use super::channel::*;
use super::*;

use crate::transport::{DisconnectReason, TransportError};
use crate::util::manyshot;

use async_std::future::Future;
use async_std::task::{ready, Context, Poll};
use std::collections::VecDeque;
use std::pin::*;

/// The `ConnectionFuture` handles all events related with a single connection.
///
/// The future needs to be constantly polled in order to drive the connection handling. It is
/// supposed to be run as isolated task. The future only resolves on error which also designates
/// the end of the connection's lifetime.
pub struct ConnectionFuture<T: TransportLayer> {
    transport: T,
    close: oneshot::Receiver<DisconnectReason>,
    channel_max_buffer_size: usize,
    channel_max_packet_size: usize,
    channels: Channels,
    request_tx: manyshot::Sender<InboundRequest>,
    request_rx: (Option<OutboundRequest>, manyshot::Receiver<OutboundRequest>),
    global_in_rx: (
        Option<Option<Vec<u8>>>,
        VecDeque<oneshot::Receiver<Result<Option<Vec<u8>>, ConnectionError>>>,
    ),
    pending_global: VecDeque<oneshot::Sender<Result<Option<Vec<u8>>, ConnectionError>>>,
}

impl<T: TransportLayer> ConnectionFuture<T> {
    pub(crate) fn new<C: ConnectionConfig>(
        config: &C,
        transport: T,
        close: oneshot::Receiver<DisconnectReason>,
        request_tx: manyshot::Sender<InboundRequest>,
        request_rx: manyshot::Receiver<OutboundRequest>,
    ) -> Self {
        Self {
            transport,
            close,
            channel_max_buffer_size: config.channel_max_buffer_size(),
            channel_max_packet_size: config.channel_max_packet_size(),
            channels: Channels::new(config.channel_max_count()),
            request_tx,
            request_rx: (None, request_rx),
            global_in_rx: (None, VecDeque::with_capacity(1)),
            pending_global: VecDeque::with_capacity(1),
        }
    }

    fn poll_events(&mut self, cx: &mut Context) -> Poll<ConnectionError> {
        loop {
            // Loop over all event sources until none of it makes progress anymore.
            // The transport shall not be flushed, but might be written to. A consolidated flush
            // will be performed afterwards. This is benefecial for networking performance as it
            // allows multiple messages to be sent in a single TCP segment (even with TCP_NODELAY)
            // and impedes traffic analysis.
            let mut made_progress = true;
            while made_progress {
                made_progress = false;
                // Poll for local connection close
                match Pin::new(&mut self.close).poll(cx) {
                    Poll::Pending => (),
                    Poll::Ready(reason) => {
                        let reason = reason.unwrap_or_default();
                        self.transport.send_disconnect(cx, reason);
                        return Poll::Ready(TransportError::DisconnectByUs(reason).into());
                    }
                }
                // Poll for global replies (outbound)
                match self.poll_global_replies(cx) {
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                    _ => (),
                }
                // Poll for global requests (outbound)
                match self.poll_outbound_requests(cx) {
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                    _ => (),
                }
                // Poll for incoming messages
                match self.poll_transport(cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                }
                // Poll for channel events
                match self.poll_channels(cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                }
            }
            // None of the previous actions shall actively flush the transport.
            // If necessary, the transport will be flushed here after all actions have eventually
            // written their output to the transport. It is necessary to loop again as some actions
            // might be pending on output and unblock as soon as buffer space becomes available
            // again. This is somewhat unlikely and will not occur unless the transport is under
            // heavy load, but it is necessary to consider this for correctness or the connection
            // will stop making progress as soon as a single notification gets lost.
            if !self.transport.flushed() {
                if let Err(e) = ready!(self.transport.poll_flush(cx)) {
                    return Poll::Ready(e.into());
                }
                continue;
            }
            // Being here means all event sources are pending and the transport is flushed.
            // Return pending as there is really nothing to do anymore for now.
            return Poll::Pending;
        }
    }

    fn poll_channels(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        // Iterate over all channel slots and poll each present channel.
        // Remove channel if the futures is ready (close has been sent _and_ received).
        for slot in self.channels.iter_mut() {
            if let Some(channel) = slot {
                match channel.poll(cx, &mut self.transport) {
                    Poll::Pending => (),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Ready(Ok(())) => *slot = None,
                }
            }
        }

        Poll::Pending
    }

    fn poll_outbound_requests(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        loop {
            let request = if let Some(request) = self.request_rx.0.take() {
                request
            } else {
                ready!(Pin::new(&mut self.request_rx.1).poll_receive(cx))
                    .ok_or(ConnectionError::Unknown)?
            };

            match request {
                OutboundRequest::Global(mut x) => {
                    let msg = MsgGlobalRequest {
                        name: x.name.clone(),
                        data: x.data.clone(), // FIXME
                        want_reply: x.reply.is_some(),
                    };
                    match self.transport.poll_send(cx, &msg) {
                        Poll::Ready(Ok(())) => {
                            if let Some(reply) = x.reply.take() {
                                self.pending_global.push_back(reply);
                            }
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                        Poll::Pending => {
                            self.request_rx.0 = Some(OutboundRequest::Global(x));
                            return Poll::Pending;
                        }
                    }
                }
                OutboundRequest::OpenSession(_) => todo!(),
                OutboundRequest::OpenDirectTcpIp(x) => {
                    if let Some(local_id) = self.channels.get_free_id() {
                        let local_ws = self.channel_max_buffer_size as u32;
                        let local_ps = self.channel_max_packet_size as u32;
                        let msg = MsgChannelOpen::<DirectTcpIp> {
                            sender_channel: local_id,
                            initial_window_size: local_ws,
                            maximum_packet_size: local_ps,
                            channel_type: x.open.clone(),
                        };
                        match self.transport.poll_send(cx, &msg) {
                            Poll::Ready(Ok(())) => {
                                let st = ChannelState::new(local_id, local_ws, local_ps, x.reply);
                                self.channels.insert(local_id, st)?;
                            }
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                            Poll::Pending => {
                                self.request_rx.0 = Some(OutboundRequest::OpenDirectTcpIp(x));
                                return Poll::Pending;
                            }
                        }
                    } else {
                        x.reply
                            .send(Ok(Err(ChannelOpenFailureReason::RESOURCE_SHORTAGE)));
                        continue;
                    }
                }
            }
        }
    }

    fn poll_global_replies(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        // Send any outstanding reply that couldn't be sent before.
        if let Some(ref reply) = self.global_in_rx.0 {
            ready!(Self::poll_send_global_reply(&mut self.transport, cx, reply))?;
            self.global_in_rx.0 = None;
        }
        // Try all other replies in the correct order (sic!).
        // Stop on the first that is not ready or store when it got ready, but couldn't be sent.
        while let Some(future) = self.global_in_rx.1.front_mut() {
            let reply = ready!(Pin::new(future).poll(cx)).unwrap_or(Ok(None))?;
            let _ = self.global_in_rx.1.pop_front();
            match Self::poll_send_global_reply(&mut self.transport, cx, &reply) {
                Poll::Ready(r) => r?,
                Poll::Pending => {
                    self.global_in_rx.0 = Some(reply);
                    return Poll::Pending;
                }
            }
        }
        Poll::Pending
    }

    fn poll_send_global_reply(
        transport: &mut T,
        cx: &mut Context,
        reply: &Option<Vec<u8>>,
    ) -> Poll<Result<(), ConnectionError>> {
        if let Some(data) = reply {
            let msg = MsgRequestSuccess {
                data: data.as_ref(),
            };
            ready!(transport.poll_send(cx, &msg))?;
        } else {
            ready!(transport.poll_send(cx, &MsgRequestFailure))?;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_transport(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        ready!(self.transport.poll_receive(cx))?;
        // MSG_CHANNEL_DATA
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelData = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_DATA ({} bytes)",
                msg.recipient_channel,
                msg.data.len()
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_data(msg.data)?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_EXTENDED_DATA
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelExtendedData = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_EXTENDED_DATA ({} bytes)",
                msg.recipient_channel,
                msg.data.len()
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_extended_data(msg.data_type_code, msg.data)?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_WINDOW_ADJUST
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelWindowAdjust = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_WINDOW_ADJUST",
                msg.recipient_channel
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_window_adjust(msg.bytes_to_add)?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_EOF
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelEof = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_EOF",
                msg.recipient_channel
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_eof()?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_CLOSE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelClose = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_CLOSE",
                msg.recipient_channel
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_close()?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_OPEN (session)
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelOpen<Session<Client>> = msg;
            log::debug!("Received MSG_CHANNEL_OPEN (session)",);
            todo!("MSG_CHANNEL_OPEN S")
        }
        // MSG_CHANNEL_OPEN (direct-tcpip)
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelOpen<DirectTcpIp> = msg;
            log::debug!("Received MSG_CHANNEL_OPEN (direct-tcpip)",);
            todo!("MSG_CHANNEL_OPEN")
        }
        // MSG_CHANNEL_OPEN_CONFIRMATION
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelOpenConfirmation = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_OPEN_CONFIRMATION",
                msg.recipient_channel
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_open_confirmation(
                msg.sender_channel,
                msg.initial_window_size,
                msg.maximum_packet_size,
            )?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_OPEN_FAILURE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelOpenFailure = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_OPEN_FAILURE",
                msg.recipient_channel
            );
            let mut channel = self.channels.remove(msg.recipient_channel)?;
            channel.push_open_failure(msg.reason)?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_REQUEST
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelRequest<&[u8]> = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_REQUEST: {}",
                msg.recipient_channel,
                msg.request
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_request(msg.specific)?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_SUCCESS
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelSuccess = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_SUCCESS",
                msg.recipient_channel
            );
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_success()?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_CHANNEL_FAILURE
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelFailure = msg;
            log::debug!("Received MSG_CHANNEL_FAILURE");
            let channel = self.channels.get(msg.recipient_channel)?;
            channel.push_failure()?;
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_GLOBAL_REQUEST
        if let Some(msg) = self.transport.decode() {
            let _: MsgGlobalRequest = msg;
            log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
            ready!(self.push_global_request(cx, msg.name, msg.data, msg.want_reply));
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_REQUEST_SUCCESS
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgRequestSuccess = msg;
            log::debug!("Received MSG_REQUEST_SUCCESS");
            if let Some(tx) = self.pending_global.pop_front() {
                tx.send(Ok(Some(msg.data.into())));
            } else {
                return Poll::Ready(Err(ConnectionError::GlobalRequestReplyUnexpected));
            }
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // MSG_REQUEST_FAILURE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgRequestFailure = msg;
            log::debug!("Received MSG_REQUEST_FAILURE");
            if let Some(tx) = self.pending_global.pop_front() {
                tx.send(Ok(None));
            } else {
                return Poll::Ready(Err(ConnectionError::GlobalRequestReplyUnexpected));
            }
            self.transport.consume();
            return Poll::Ready(Ok(()));
        }
        // Otherwise try to send MSG_UNIMPLEMENTED and return error.
        self.transport.send_unimplemented(cx);
        Poll::Ready(Err(TransportError::MessageUnexpected.into()))
    }

    fn push_global_request(
        &mut self,
        cx: &mut Context,
        name: String,
        data: Vec<u8>,
        want_reply: bool,
    ) -> Poll<Result<(), ConnectionError>> {
        let mut req = GlobalRequest {
            name,
            data,
            reply: None,
        };
        if want_reply {
            let (tx, rx) = oneshot::channel();
            req.reply = Some(tx);
            let req = InboundRequest::Global(req);
            ready!(self.request_tx.poll_send(cx, req)).ok_or(ConnectionError::Unknown)?;
            self.global_in_rx.1.push_back(rx);
            Poll::Ready(Ok(()))
        } else {
            let req = InboundRequest::Global(req);
            ready!(self.request_tx.poll_send(cx, req)).ok_or(ConnectionError::Unknown)?;
            Poll::Ready(Ok(()))
        }
    }
}

impl<T: TransportLayer> Future for ConnectionFuture<T> {
    type Output = ConnectionError;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let self_ = Pin::into_inner(self);
        let e = ready!(self_.poll_events(cx));
        log::debug!("Connection terminated due to {:?}", e);
        self_.terminate(e);
        Poll::Ready(e)
    }
}

impl<T: TransportLayer> Terminate for ConnectionFuture<T> {
    /// Deliver a `ConnectionError` to all dependant users of this this connection (tasks waiting
    /// on connection requests or channel I/O).
    ///
    /// This shall be the last thing to happen and has great similarity with `Drop` except that
    /// it distributes an error.
    fn terminate(&mut self, e: ConnectionError) {
        if let Some(x) = self.request_rx.0.take() {
            match x {
                OutboundRequest::Global(mut x) => {
                    x.reply.take().map(|x| x.send(Err(e))).unwrap_or(())
                }
                OutboundRequest::OpenSession(_) => todo!(),
                OutboundRequest::OpenDirectTcpIp(_) => todo!(),
            }
        }
        while let Some(x) = self.pending_global.pop_front() {
            x.send(Err(e))
        }
        // FIXME tx
        self.channels.terminate(e);
    }
}
