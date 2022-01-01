use super::channel::direct_tcpip::DirectTcpIp;
use super::channel::session;
use super::channel::session::*;
use super::channel::OpenFailure;
use super::channel::{Channel, ChannelState};
use super::config::ConnectionConfig;
use super::error::ConnectionError;
use super::global::*;
use super::handler::ConnectionHandler;
use super::msg::*;
use super::request::*;
use crate::connection::channel::PollResult;
use crate::ready;
use crate::transport::Message;
use crate::transport::{Transport, DisconnectReason, TransportError};
use crate::util::codec::*;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;

pub struct ConnectionState {
    /// Config
    config: Arc<ConnectionConfig>,
    /// Callbacks for incoming events
    handler: Box<dyn ConnectionHandler>,
    /// Underlying transport
    transport: Transport,
    /// Next request to process
    requests_head: Option<Request>,
    /// Async bounded queue of requests to process
    requests_queue: mpsc::Receiver<Request>,
    /// Ordered list of transmitted global requests awaiting reply
    requests_replies: VecDeque<oneshot::Sender<Result<Vec<u8>, ()>>>,
    /// Next global request reply ready for transmission
    replies_head: Option<Result<Vec<u8>, ()>>,
    /// Ordererd list of global requests eventually ready for transmission
    replies_queue: VecDeque<oneshot::Receiver<Vec<u8>>>,
    /// List of active channels (index is local channel id)
    channels: Vec<Option<Box<dyn ChannelState>>>,
    /// List of remote channel ids that still need to be rejected due to resource shortage
    channels_reject: VecDeque<(u32, OpenFailure)>,
    /// Canary indicating whether all handles on this connection have been dropped
    close_tx: oneshot::Sender<()>,
    /// Distribution point for eventually occuring connection error
    error_tx: watch::Sender<Option<Arc<ConnectionError>>>,
    error_rx: watch::Receiver<Option<Arc<ConnectionError>>>,
}

impl ConnectionState {
    /// Create a new state with config and transport.
    pub fn new(
        config: &Arc<ConnectionConfig>,
        handler: Box<dyn ConnectionHandler>,
        transport: Transport,
        requests: mpsc::Receiver<Request>,
        close_tx: oneshot::Sender<()>,
        error_tx: watch::Sender<Option<Arc<ConnectionError>>>,
        error_rx: watch::Receiver<Option<Arc<ConnectionError>>>,
    ) -> Self {
        Self {
            config: config.clone(),
            handler,
            transport,
            requests_head: None,
            requests_queue: requests,
            requests_replies: VecDeque::new(),
            replies_head: None,
            replies_queue: VecDeque::new(),
            channels: Vec::new(),
            channels_reject: VecDeque::new(),
            close_tx,
            error_tx,
            error_rx,
        }
    }

    /// Poll the connection and make progress.
    ///
    /// This method returns `Ready(Ok(()))` in case all work has been done. It is safe
    /// to convert this into `Pending` (the [Context] has been registered for wakeup on all relevant
    /// events). It returns early with `Pending` in case any transport operation returned `Pending`.
    /// In this case the [Context] is only guaranteed to be registered for wakeup on transport
    /// readyness although it might have been registered for other events as well. In case such an
    /// event causes wakeup the next invocation will most likely get blocked on the non-ready
    /// transport again (this is unfortunate but doesn't cause any harm).
    ///
    /// The order of calls within this function has been designed by priority: The close event is
    /// checked first. Secondly, it is tried to read from the transport (this is strictly necessary
    /// as not reading all messages in co-occurence with key re-exchange would lead to a deadlock
    /// situation; somewhat a design flaw in the specification).
    ///
    /// If necessary, the transport will be flushed here after all actions have eventually
    /// written their output to the transport. This is benefecial for network performance as it
    /// allows multiple messages to be sent in a single TCP segment (even with TCP_NODELAY) and
    /// impedes traffic analysis.
    fn poll_everything(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        ready!(self.poll_close(cx))?;
        ready!(self.poll_transport(cx))?;
        ready!(self.poll_replies(cx))?;
        ready!(self.poll_requests(cx))?;
        ready!(self.poll_channels(cx))?;
        ready!(self.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    /// Poll whether the connection shall be closed.
    ///
    /// Returns `Ready(Err(_))` when close is desired and `Ready(Ok(())` else.
    ///
    /// The connection shall be closed when either the user called close on the connection handle
    /// or dropped it or when polling the connection handler object returns with [Poll::Ready].
    /// The function then returns a `disconnect by application` error which shall be handled and
    /// distributed by the caller. The connection task shall then be terminated.
    fn poll_close(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if self.close_tx.poll_closed(cx).is_ready() || self.handler.poll(cx).is_ready() {
            let e = TransportError::DisconnectByUs(DisconnectReason::BY_APPLICATION);
            Poll::Ready(Err(e.into()))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    /// Poll the transport for incoming messages.
    ///
    /// Returns Ready(Ok(())) when all available messages have been dispatched.
    /// Returns Ready(Err(_)) on error.
    /// Returns Pending when the transport is currently busy (due to key re-exchange).
    ///
    /// NB: Any message that is received gets dispatched. The dispatch mechanism does not cause
    /// the operation to return `Pending`. This is important to avoid deadlock situations!
    fn poll_transport(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        /// Try to get a channel state by local id from list of channels.
        ///
        /// Throws error if channel id is invalid.
        /// Returns a [std::sync::MutexGuard<DirectTcpIpState>] on success.
        /// Drop the mutex guard as soon as possible (i.e. by using `wake` macro)!
        macro_rules! channel {
            ($state:ident, $lid:ident) => {
                $state
                    .channels
                    .get_mut($lid as usize)
                    .and_then(|x| x.as_mut())
                    .ok_or(ConnectionError::ChannelInvalid)?
            };
        }
        macro_rules! channel_remove {
            ($state:ident, $lid:ident) => {
                $state
                    .channels
                    .get_mut($lid as usize)
                    .and_then(|x| x.take())
                    .ok_or(ConnectionError::ChannelInvalid)?
            };
        }
        macro_rules! channel_replace {
            ($state:ident, $lid:ident, $f:expr) => {
                let c = $state
                    .channels
                    .get_mut($lid as usize)
                    .and_then(|x| x.take())
                    .ok_or(ConnectionError::ChannelInvalid)?;
                $state.channels[$lid as usize] = Some($f(c)?);
            };
        }

        // Poll the transport for the next message available.
        while let Some(buf) = ready!(self.transport.poll_next(cx))? {
            // Dispatch message (must not block; message MUST be dispatched)
            match *buf.get(0).unwrap_or(&0) {
                <MsgChannelOpen as Message>::NUMBER => {
                    let msg: MsgChannelOpen = SshCodec::decode(buf)?;
                    log::debug!("<< {:?}", msg);
                    if let Some(lid) = self.alloc_channel_id() {
                        match msg.name.as_str() {
                            SessionClient::NAME => {
                                let (cst, req) = session::open(&self.config, &msg, lid)?;
                                self.handler.on_session_request(req);
                                self.channels[lid as usize] = Some(cst);
                            }
                            DirectTcpIp::NAME => {
                                let (cst, req) = DirectTcpIp::open_in(&self.config, &msg, lid)?;
                                self.handler.on_direct_tcpip_request(req);
                                self.channels[lid as usize] = Some(cst);
                            }
                            _ => {
                                let e = OpenFailure::UNKNOWN_CHANNEL_TYPE;
                                self.channels_reject.push_back((msg.sender_channel, e));
                            }
                        }
                    } else {
                        let e = OpenFailure::RESOURCE_SHORTAGE;
                        self.channels_reject.push_back((msg.sender_channel, e));
                    }
                }
                <MsgChannelOpenConfirmation as Message>::NUMBER => {
                    let msg: MsgChannelOpenConfirmation = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    let rid = msg.sender_channel;
                    let rws = msg.initial_window_size;
                    let rps = msg.maximum_packet_size;
                    log::debug!("<< {:?}", msg);
                    channel_replace!(self, lid, |x: Box<dyn ChannelState>| x
                        .on_open_confirmation(rid, rws, rps));
                }
                <MsgChannelOpenFailure as Message>::NUMBER => {
                    let msg: MsgChannelOpenFailure = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel_remove!(self, lid).on_open_failure(msg.reason)?;
                }
                <MsgChannelData as Message>::NUMBER => {
                    let msg: MsgChannelData = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_data(msg.data)?;
                }
                <MsgChannelExtendedData as Message>::NUMBER => {
                    let msg: MsgChannelExtendedData = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_ext_data(msg.data_type_code, msg.data)?;
                }
                <MsgChannelWindowAdjust as Message>::NUMBER => {
                    let msg: MsgChannelWindowAdjust = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_window_adjust(msg.bytes_to_add)?;
                }
                <MsgChannelEof as Message>::NUMBER => {
                    let msg: MsgChannelEof = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_eof()?;
                }
                <MsgChannelClose as Message>::NUMBER => {
                    let msg: MsgChannelClose = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_close()?;
                }
                <MsgChannelRequest<()> as Message>::NUMBER => {
                    let msg: MsgChannelRequest<&[u8]> = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    let typ = msg.request;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_request(typ, msg.specific, msg.want_reply)?;
                }
                <MsgChannelSuccess as Message>::NUMBER => {
                    let msg: MsgChannelSuccess = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel_replace!(self, lid, |x: Box<dyn ChannelState>| x.on_success());
                }
                <MsgChannelFailure as Message>::NUMBER => {
                    let msg: MsgChannelFailure = SshCodec::decode(buf)?;
                    let lid = msg.recipient_channel;
                    log::debug!("<< {:?}", msg);
                    channel!(self, lid).on_failure()?;
                }
                <MsgGlobalRequest as Message>::NUMBER => {
                    let msg: MsgGlobalRequest = SshCodec::decode(buf)?;
                    log::debug!("<< {:?}", msg);
                    if msg.want_reply {
                        let (s, r) = oneshot::channel();
                        self.replies_queue.push_back(r);
                        let request =
                            GlobalRequestWantReply::new(msg.name.into(), msg.data.into(), s);
                        self.handler.on_request_want_reply(request);
                    } else {
                        let request = GlobalRequest::new(msg.name.into(), msg.data.into());
                        self.handler.on_request(request);
                    }
                }
                <MsgRequestSuccess as Message>::NUMBER => {
                    let msg: MsgRequestSuccess = SshCodec::decode(buf)?;
                    log::debug!("<< {:?}", msg);
                    let data = msg.data.into();
                    self.requests_replies
                        .pop_front()
                        .ok_or(ConnectionError::GlobalReplyUnexpected)
                        .map(|s| s.send(Ok(data)).unwrap_or(()))?;
                }
                <MsgRequestFailure as Message>::NUMBER => {
                    let msg: MsgRequestFailure = SshCodec::decode(buf)?;
                    log::debug!("<< {:?}", msg);
                    self.requests_replies
                        .pop_front()
                        .ok_or(ConnectionError::GlobalReplyUnexpected)
                        .map(|s| s.send(Err(())).unwrap_or(()))?
                }
                x => {
                    log::debug!("<< Unimplemented message type {}", x);
                    // MSG_UNIMPLEMENTED(3) may be sent even during key re-exchange.
                    // In case the following call returns `Pending` this can only be due to output
                    // buffer congestion, thus a later retry here will succeed and not lead to a
                    // deadlock situation like with other message types.
                    //ready!(self.transport.poll_send_unimplemented(cx)?);
                    todo!() // FIXME
                }
            }

            // Consume the message buffer after successful dispatch!
            self.transport.consume_next()?;
        }

        Poll::Ready(Ok(()))
    }

    /// Try sending ready global request replies (if any).
    ///
    /// The replies must be sent in original order even if they get ready in a different order.
    /// The function will first try to send from `self.replies_head` which eventually contains
    /// a reply that was already available last time but the socket was blocking.
    /// It will then proceed trying to transmit from the queue until it encounters a non-ready reply.
    /// The function will only return `Pending` on a blocking transport (not on a non-ready reply).
    /// In case there were one or more pending non-ready replies it is guaranteed that the [Context]
    /// is registered for wakeup as soon as the first of these replies becomes ready.
    fn poll_replies(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if let Some(reply) = &self.replies_head {
            match reply {
                Ok(data) => {
                    let msg = MsgRequestSuccess { data: &data };
                    ready!(self.transport.poll_send(cx, &msg))?;
                }
                Err(_) => {
                    let msg = MsgRequestFailure;
                    ready!(self.transport.poll_send(cx, &msg))?;
                }
            }
            self.replies_head = None;
        }

        while let Some(mut x) = self.replies_queue.front_mut() {
            if let Poll::Ready(reply) = Future::poll(Pin::new(&mut x), cx) {
                let _ = self.replies_queue.pop_front();
                match reply {
                    Ok(data) => {
                        let msg = MsgRequestSuccess { data: &data };
                        if self.transport.poll_send(cx, &msg)?.is_pending() {
                            self.replies_head = Some(Ok(data));
                            return Poll::Pending;
                        }
                    }
                    Err(_) => {
                        let msg = MsgRequestFailure;
                        if self.transport.poll_send(cx, &msg)?.is_pending() {
                            self.replies_head = Some(Err(()));
                            return Poll::Pending;
                        }
                    }
                }
            } else {
                break;
            }
        }

        // Save memory by shrinking vector to fit
        self.replies_queue.shrink_to_fit();
        Poll::Ready(Ok(()))
    }

    /// Try processing local requests (global and channel open, if any).
    ///
    /// This function returns `Ready(Ok(_))` if all requests have been processed and `Pending` on
    /// a blocking transport. The [Context] is registered for wakeup when the next request becomes
    /// available or when the transport gets ready (in case it blocked).
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        loop {
            let cr = if let Some(cr) = self.requests_head.take() {
                cr
            } else {
                match self.requests_queue.poll_recv(cx) {
                    Poll::Pending => return Poll::Ready(Ok(())),
                    Poll::Ready(cr) => cr.ok_or(ConnectionError::Dropped)?,
                }
            };
            match cr {
                Request::Global { name, data, reply } => {
                    let msg = MsgGlobalRequest {
                        name: &name,
                        data: &data.as_ref(),
                        want_reply: reply.is_some(),
                    };
                    match self.transport.poll_send(cx, &msg) {
                        Poll::Ready(r) => r?,
                        Poll::Pending => {
                            let cr = Request::Global { name, data, reply };
                            self.requests_head = Some(cr);
                            return Poll::Pending;
                        }
                    }
                    if let Some(reply) = reply {
                        self.requests_replies.push_back(reply);
                    }
                }
                Request::OpenSession { reply } => {
                    if let Some(lid) = self.alloc_channel_id() {
                        let error_rx = self.error_rx.clone();
                        let channel = SessionClient::open(&self.config, lid, error_rx, reply);
                        self.channels[lid as usize] = Some(channel);
                    } else {
                        let e = OpenFailure::RESOURCE_SHORTAGE;
                        reply.send(Err(e)).unwrap_or(());
                    }
                }
                Request::OpenDirectTcpIp { reply, params } => {
                    if let Some(lid) = self.alloc_channel_id() {
                        let channel = DirectTcpIp::open_out(&self.config, lid, reply, params)?;
                        self.channels[lid as usize] = Some(channel);
                    } else {
                        let e = OpenFailure::RESOURCE_SHORTAGE;
                        reply.send(Err(e)).unwrap_or(());
                    }
                }
            }
        }
    }

    /// Try processing channel events (if any).
    ///
    /// This function first tries to transmit pending channel rejections. It then proceeds polling
    /// each active channel. In case all work has been done and there is nothing else to do it will
    /// return `Ready(Ok(()))`. The [Context] will have been registered for wakeup on all channel
    /// events. The function returns `Pending` only in case the transport blocked. It then will have
    /// the [Context] registered for wakeup when the transport becomes ready again.
    ///
    /// The function also removes all closed channels from the channel list (a channel is closed
    /// if MSG_CHANNEL_CLOSE has been sent and received) and drops its handles on them. The channel
    /// state get finally freed as soon as the last handle on it gets dropped which might not be
    /// the one that is dropped here depending on which side initiated the close procedure.
    fn poll_channels(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        while let Some((rid, e)) = self.channels_reject.front() {
            let msg = MsgChannelOpenFailure::new(*rid, *e);
            ready!(self.transport.poll_send(cx, &msg))?;
            let _ = self.channels_reject.pop_front();
        }

        let mut empty_tail_elements = 0;

        for channel in &mut self.channels {
            if let Some(ref mut c) = channel {
                match ready!(c.poll_with_transport(cx, &mut self.transport))? {
                    PollResult::Noop => (),
                    PollResult::Closed => *channel = None,
                    PollResult::Replace(x) => *channel = Some(x),
                }
            }
            if channel.is_none() {
                empty_tail_elements += 1;
            } else {
                empty_tail_elements = 0;
            }
        }

        // Truncate the channel list if it contains empty channel slots at the end.
        if empty_tail_elements > 0 {
            let keep = self.channels.len() - empty_tail_elements;
            self.channels.truncate(keep);
            self.channels.shrink_to_fit();
        }

        Poll::Ready(Ok(()))
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        self.transport.poll_flush(cx).map_err(Into::into)
    }

    fn alloc_channel_id(&mut self) -> Option<u32> {
        for (id, slot) in self.channels.iter().enumerate() {
            if slot.is_none() {
                return Some(id as u32);
            }
        }
        let id = self.channels.len();
        if id < self.config.channel_max_count as usize {
            self.channels.push(None);
            Some(id as u32)
        } else {
            None
        }
    }
}

impl Future for ConnectionState {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        log::trace!("ConnectionState.poll()");
        let self_ = Pin::into_inner(self);
        if let Err(e) = ready!(self_.poll_everything(cx)) {
            // In case an error occurs do the following in this order:
            //   1. Replace and thereby drop the connection handler
            //   2. Dispatch the error to all channels and drop the handles on them
            //   3. Broadcast the error to all other users of the connection
            //   4. Return `Poll::Ready` and thereby terminate the connection task
            let e = Arc::new(e);
            std::mem::replace(&mut self_.handler, Box::new(())).on_error(&e);
            for channel in &mut self_.channels {
                if let Some(channel) = channel.take() {
                    channel.on_error(&e);
                }
            }
            let _ = self_.error_tx.send(Some(e));
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl std::fmt::Debug for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectionState {{ ... }}")
    }
}
