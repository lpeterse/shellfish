use super::channel::state::ChannelState;
use super::channel::ChannelHandle;
use super::channel::ChannelOpenFailure;
use super::channel::ChannelOpenRequest;
use super::config::ConnectionConfig;
use super::error::ConnectionError;
use super::global::*;
use super::handler::ConnectionHandler;
use super::msg::*;
use super::request::*;
use crate::ready;
use crate::transport::{DisconnectReason, GenericTransport, Transport, TransportError};
use crate::util::codec::*;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
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
    transport: GenericTransport,
    /// Next request to process
    requests_head: Option<ConnectionRequest>,
    /// Async bounded queue of requests to process
    requests_queue: mpsc::Receiver<ConnectionRequest>,
    /// Ordered list of transmitted global requests awaiting reply
    requests_replies: VecDeque<oneshot::Sender<Result<Vec<u8>, ()>>>,
    /// Next global request reply ready for transmission
    replies_head: Option<Result<Vec<u8>, ()>>,
    /// Ordererd list of global requests eventually ready for transmission
    replies_queue: VecDeque<oneshot::Receiver<Vec<u8>>>,
    /// List of active channels (index is local channel id)
    channels: Vec<Option<Arc<Mutex<ChannelState>>>>,
    /// List of remote channel ids that still need to be rejected due to resource shortage
    channels_rejections: VecDeque<u32>,
    /// Canary indicating whether all handles on this connection have been dropped
    close: oneshot::Sender<()>,
    /// Distribution point for eventually occuring connection error
    error: watch::Sender<Option<ConnectionError>>,
}

impl ConnectionState {
    /// Create a new state with config and transport.
    pub fn new(
        config: &Arc<ConnectionConfig>,
        handler: Box<dyn ConnectionHandler>,
        transport: GenericTransport,
        requests: mpsc::Receiver<ConnectionRequest>,
        close: oneshot::Sender<()>,
        error: watch::Sender<Option<ConnectionError>>,
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
            channels_rejections: VecDeque::new(),
            close,
            error,
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
        if self.close.poll_closed(cx).is_ready() || self.handler.poll(cx).is_ready() {
            let e = TransportError::DisconnectByUs(DisconnectReason::BY_APPLICATION);
            Poll::Ready(Err(e.into()))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    /// Poll the transport for incoming messages.
    ///
    ///     - Returns `Ready(Ok(()))` when all available messages have been dispatched.
    ///     - Returns `Ready(Err(_))` on error.
    ///     - Returns `Pending` when the transport is currently busy (due to key re-exchange).
    ///
    /// NB: Any message that is received gets dispatched. The dispatch mechanism does not cause
    /// the operation to return `Pending`. This is important to avoid deadlock situations!
    fn poll_transport(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        /// Try to get a channel state by local id from list of channels.
        ///     - Throws error if channel id is invalid.
        ///     - Returns a [std::sync::MutexGuard<ChannelState>] on success.
        ///     - Drop the mutex guard as soon as possible (i.e. by using `wake` macro)!
        macro_rules! channel {
            ($state:ident, $lid:ident) => {
                $state
                    .channels
                    .get($lid as usize)
                    .and_then(|x| x.as_ref())
                    .ok_or(ConnectionError::ChannelIdInvalid)?
                    .lock()
                    .unwrap()
            };
        }
        /// Consume a [std::sync::MutexGuard<ChannelState>] and wake the channel if necessary.
        ///     - It takes the waker from the channel (if present), then releases the lock by
        ///       dropping the mutex guard and _then_ actually wakes the waker. This is important
        //        in order to avoid lock contention on concurrent execution!
        macro_rules! wake {
            ($chan:ident) => {
                let w = $chan.outer_task_waker.take();
                drop($chan);
                if let Some(waker) = w {
                    waker.wake()
                }
            };
        }

        loop {
            // Poll the transport for the next message available.
            // This is the loop's only exit point (except for errors).
            let buf = match ready!(self.transport.rx_peek(cx))? {
                None => return Poll::Ready(Ok(())),
                Some(buf) => buf,
            };
            // Dispatch the different message types.
            if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelOpen = msg;
                if let Some(lid) = self.alloc_channel_id() {
                    log::debug!("Channel {}: Received MSG_CHANNEL_OPEN ({})", lid, msg.name);
                    // Create a new channel state object and call the handler with a corresponding
                    // channel open request. The channel is not open until channel open confirmation
                    // has been sent (happens when channel is polled later).
                    let (s, r) = oneshot::channel();
                    let lws = self.config.channel_max_buffer_size;
                    let lps = self.config.channel_max_packet_size;
                    let rid = msg.sender_channel;
                    let rws = msg.initial_window_size;
                    let rps = msg.maximum_packet_size;
                    let cst = ChannelState::new_inbound(lid, lws, lps, rid, rws, rps, false, r); // FIXME
                    let cst = Arc::new(Mutex::new(cst));
                    let req = ChannelOpenRequest {
                        name: msg.name,
                        data: msg.data,
                        chan: ChannelHandle::new(cst.clone()),
                        resp: s,
                    };
                    self.channels[lid as usize] = Some(cst);
                    self.handler.on_open_request(req)
                } else {
                    log::debug!("Channel _: Rejecting MSG_CHANNEL_OPEN ({})", msg.name);
                    self.channels_rejections.push_back(msg.sender_channel);
                }
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelOpenConfirmation = msg;
                let lid = msg.recipient_channel;
                let rid = msg.sender_channel;
                let rws = msg.initial_window_size;
                let rps = msg.maximum_packet_size;
                log::debug!("Channel {}: Received MSG_CHANNEL_OPEN_CONFIRMATION", lid);
                let handle = ChannelHandle::new(
                    self.channels
                        .get(lid as usize)
                        .and_then(|x| x.as_ref())
                        .ok_or(ConnectionError::ChannelIdInvalid)?
                        .clone(),
                );
                channel!(self, lid).push_open_confirmation(rid, rws, rps, handle)?;
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelOpenFailure = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_OPEN_FAILURE", lid);
                channel!(self, lid).push_open_failure(msg.reason)?;
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelData = msg;
                let lid = msg.recipient_channel;
                let len = msg.data.len();
                log::debug!("Channel {}: Received MSG_CHANNEL_DATA ({} bytes)", lid, len);
                let mut channel = channel!(self, lid);
                channel.push_data(msg.data)?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelExtendedData = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_EXTENDED_DATA", lid);
                let mut channel = channel!(self, lid);
                channel.push_extended_data(msg.data_type_code, msg.data)?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelWindowAdjust = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_WINDOW_ADJUST", lid);
                let mut channel = channel!(self, lid);
                channel.push_window_adjust(msg.bytes_to_add)?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelEof = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_EOF", lid);
                let mut channel = channel!(self, lid);
                channel.push_eof()?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelClose = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_CLOSE", lid);
                let mut channel = channel!(self, lid);
                channel.push_close()?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelRequest<&[u8]> = msg;
                let lid = msg.recipient_channel;
                let typ = msg.request;
                log::debug!("Channel {}: Received MSG_CHANNEL_REQUEST: {:?}", lid, typ);
                let mut channel = channel!(self, lid);
                channel.push_request(typ, msg.specific, msg.want_reply)?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelSuccess = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_SUCCESS", lid);
                let mut channel = channel!(self, lid);
                channel.push_success()?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgChannelFailure = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_FAILURE", lid);
                let mut channel = channel!(self, lid);
                channel.push_failure()?;
                wake!(channel);
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgGlobalRequest = msg;
                log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
                if msg.want_reply {
                    let (s, r) = oneshot::channel();
                    self.replies_queue.push_back(r);
                    let request = GlobalRequestWantReply::new(msg.name.into(), msg.data.into(), s);
                    self.handler.on_request_want_reply(request);
                } else {
                    let request = GlobalRequest::new(msg.name.into(), msg.data.into());
                    self.handler.on_request(request);
                }
                Ok(())
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgRequestSuccess = msg;
                log::debug!("Received MSG_REQUEST_SUCCESS");
                let data = msg.data.into();
                self.requests_replies
                    .pop_front()
                    .ok_or(ConnectionError::GlobalReplyUnexpected)
                    .map(|s| s.send(Ok(data)).unwrap_or(()))
            } else if let Some(msg) = SshCodec::decode(buf) {
                let _: MsgRequestFailure = msg;
                log::debug!("Received MSG_REQUEST_FAILURE");
                self.requests_replies
                    .pop_front()
                    .ok_or(ConnectionError::GlobalReplyUnexpected)
                    .map(|s| s.send(Err(())).unwrap_or(()))
            } else {
                unimplemented!() // FIXME
            }?;

            // Consume the message buffer after successful dispatch!
            self.transport.rx_consume()?;
        }
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
                ConnectionRequest::Global { name, data, reply } => {
                    let msg = MsgGlobalRequest {
                        name: &name,
                        data: &data.as_ref(),
                        want_reply: reply.is_some(),
                    };
                    match self.transport.poll_send(cx, &msg) {
                        Poll::Ready(r) => r?,
                        Poll::Pending => {
                            let cr = ConnectionRequest::Global { name, data, reply };
                            self.requests_head = Some(cr);
                            return Poll::Pending;
                        }
                    }
                    if let Some(reply) = reply {
                        self.requests_replies.push_back(reply);
                    }
                }
                ConnectionRequest::Open { name, data, reply } => {
                    if let Some(lid) = self.alloc_channel_id() {
                        let lws = self.config.channel_max_buffer_size as u32;
                        let lps = self.config.channel_max_packet_size as u32;
                        let msg = MsgChannelOpen {
                            name: name,
                            sender_channel: lid,
                            initial_window_size: lws,
                            maximum_packet_size: lps,
                            data: data.clone(),
                        };
                        match self.transport.poll_send(cx, &msg) {
                            Poll::Ready(r) => r?,
                            Poll::Pending => {
                                let cr = ConnectionRequest::Open { name, data, reply };
                                self.requests_head = Some(cr);
                                return Poll::Pending;
                            }
                        }
                        let cst = ChannelState::new_outbound(lid, lws, lps, true, reply); // FIXME
                        self.channels[lid as usize] = Some(Arc::new(Mutex::new(cst)));
                    } else {
                        let e = ChannelOpenFailure::RESOURCE_SHORTAGE;
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
        while let Some(rid) = self.channels_rejections.front() {
            let msg = MsgChannelOpenFailure::new(*rid, ChannelOpenFailure::RESOURCE_SHORTAGE);
            ready!(self.transport.poll_send(cx, &msg))?;
            let _ = self.channels_rejections.pop_front();
        }

        let mut empty_tail_elements = 0;

        for channel in &mut self.channels {
            let mut closed = false;
            let mut waker = None;
            if let Some(ref channel) = channel {
                empty_tail_elements = 0;
                let mut channel = channel.lock().unwrap();
                ready!(channel.poll_with_transport(cx, &mut self.transport))?;
                closed = channel.close_sent && channel.close_rcvd;
                waker = channel.take_outer_waker();
                drop(channel); // Release Mutex lock! (just to make it more clear)
            } else {
                empty_tail_elements += 1;
            }
            if closed {
                // The channel is actually an `Arc`. We just detach it from the connection.
                // It is freed as soon as the `ChannelHandle` gets dropped.
                *channel = None;
            }
            if let Some(waker) = waker {
                // The waker is only present if the `outer task` demanded being woken up on the
                // current condition. The Mutex lock on the channel has been release already
                // (important in order to avoid Mutex contention on concurrent execution).
                waker.wake()
            }
        }

        if empty_tail_elements > 0 {
            // Truncate the channel list if it contains empty channel slots at the end.
            let keep = self.channels.len() - empty_tail_elements;
            self.channels.truncate(keep);
            self.channels.shrink_to_fit();
        }

        Poll::Ready(Ok(()))
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        self.transport.tx_flush(cx).map_err(Into::into)
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
        let self_ = Pin::into_inner(self);
        if let Err(e) = ready!(self_.poll_everything(cx)) {
            let _ = self_.error.send(Some(e));
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
