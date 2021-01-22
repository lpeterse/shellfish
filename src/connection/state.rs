use super::channel::*;
use super::*;

use crate::transport::{DisconnectReason, GenericTransport, TransportError};
use crate::util::check;

use std::task::{ready, Context, Poll, Waker};
use std::collections::VecDeque;
use std::sync::Arc;

/// The connection state is shared between a house-keeping task (inner task) and the user task
/// (outer tasks). The state is protected by an Arc<Mutex<>> (see wrapper structs).
///
/// The inner task sees the state wrapped as a `ConnectionFuture` which only exposes the
/// `Future::poll` method. The outer tasks see the state wrapped as a `Connection`.
///
/// The code is carefully designed in order to reduce the lock contention to a minimum. Most access
/// operations shall find the Mutex unlocked. std::sync::Mutex is highly portable although there
/// are even more efficient implementations (parking_lot) especially for the not-contented case.
/// For now (Feb 2020), we'll sacrifice that potential performance increase in favour of simplicity.
/// We'll automatically benefit from any improvements in std in the future.
pub struct ConnectionState {
    config: Arc<ConnectionConfig>,
    handler: Box<dyn ConnectionHandler>,
    transport: GenericTransport,
    channels: ChannelList,
    local_requests: VecDeque<GlobalRequest>,
    local_replies: VecDeque<oneshot::Sender<Result<Vec<u8>, ConnectionError>>>,
    remote_requests: VecDeque<GlobalRequest>,
    remote_replies: VecDeque<oneshot::Receiver<Result<Vec<u8>, ConnectionError>>>,
    inner_task_wake: bool,
    outer_task_wake: bool,
    inner_task_waker: Option<Waker>,
    outer_task_waker: Option<Waker>,
    result: Option<Result<DisconnectReason, ConnectionError>>,
}

impl ConnectionState {
    /// Create a new state with config and transport.
    ///
    /// The config `Arc` will be cloned. All queues are initialised with zero
    /// capacity in expectation that they will never be used.
    pub fn new(
        config: &Arc<ConnectionConfig>,
        handler: Box<dyn ConnectionHandler>,
        transport: GenericTransport,
    ) -> Self {
        Self {
            config: config.clone(),
            handler,
            transport,
            channels: ChannelList::new(&config),
            local_requests: VecDeque::with_capacity(0),
            local_replies: VecDeque::with_capacity(0),
            remote_requests: VecDeque::with_capacity(0),
            remote_replies: VecDeque::with_capacity(0),
            inner_task_wake: false,
            outer_task_wake: false,
            inner_task_waker: None,
            outer_task_waker: None,
            result: None,
        }
    }

    /// This operation shall be polled by the inner task. It coordinates all data transmission and
    /// requests and drives the transport layer mechanisms (like kex).
    ///
    /// It always returns `Pending` unless the connection terminates.
    pub fn poll(&mut self, cx: &mut Context) -> Poll<Result<DisconnectReason, ConnectionError>> {
        self.register_inner_task(cx);
        // Check result (ready when connection shall be terminated)
        if let Poll::Ready(result) = self.poll_result(cx) {
            return Poll::Ready(result);
        }
        // Try flushing transport and consume inbound messages
        if let Poll::Ready(x) = self.poll_transport(cx) {
            x?
        }
        // Try sending pending global replies
        if let Poll::Ready(x) = self.poll_global_replies(cx) {
            x?
        }
        // Try sending pending global requests
        if let Poll::Ready(x) = self.poll_global_requests(cx) {
            x?
        }
        // Try processing channel events
        if let Poll::Ready(x) = self.channels.poll(cx, &mut self.transport) {
            x?
        }
        // The 3 previous actions shall not actively flush the transport.
        // If necessary, the transport will be flushed here after all actions have eventually
        // written their output to the transport. This is benefecial for network performance as it
        // allows multiple messages to be sent in a single TCP segment (even with TCP_NODELAY) and
        // impedes traffic analysis.
        ready!(self.transport.tx_flush(cx))?;
        Poll::Pending
    }

    /// Take the next queued inbound request (if present).
    ///
    /// Taking a request flags the inner task for wakeup, but doesn't actually wake it up right away.
    /// Use `inner_task_waker` afterwards to obtain a `Waker` and use it after releasing the lock!
    pub fn next(&mut self) -> Option<ConnectionRequest> {
        if let Some(request) = self.remote_requests.pop_front() {
            self.flag_inner_task_for_wakeup();
            return Some(ConnectionRequest::Global(request));
        }
        if let Some(request) = self.channels.take_open_request() {
            self.flag_inner_task_for_wakeup();
            return Some(ConnectionRequest::ChannelOpen(request));
        }
        None
    }

    /// Get the connection result (if present).
    ///
    /// `Some(Ok(_))` means the peer sent a disconnect. This is usually not an error condition.
    /// `Some(Err(_))` means that the connection terminated/shall terminate for any other reason.
    pub fn result(&self) -> Option<Result<DisconnectReason, ConnectionError>> {
        self.result.clone()
    }

    /// Enqueue a global request (outbound).
    ///
    /// This flags the inner task for wakeup, but doesn't actually wake it up right away.
    /// Use `inner_task_waker` afterwards to obtain a `Waker` and use it after releasing the lock!
    pub fn request(&mut self, name: String, data: Vec<u8>) {
        let request = GlobalRequest::new(name, data);
        self.local_requests.push_back(request);
        self.flag_inner_task_for_wakeup();
    }

    /// Enqueue a global request (outbound) and return a future expecting the reply.
    ///
    /// The future resolves immediately with a `ConnectionError` in case the connection is dead.
    ///
    /// Otherwise, this flags the inner task for wakeup, but doesn't actually wake it up right away.
    /// Use `inner_task_waker` afterwards to obtain a `Waker` and use it after releasing the lock!
    pub fn request_want_reply(&mut self, name: String, data: Vec<u8>) -> GlobalReplyFuture {
        if let Some(ref result) = self.result {
            let (tx, rx) = oneshot::channel();
            tx.send(Err(result.clone().into()));
            return GlobalReplyFuture::new(rx);
        }
        let (request, reply) = GlobalRequest::new_want_reply(name, data);
        self.local_requests.push_back(request);
        self.flag_inner_task_for_wakeup();
        reply
    }

    /// Enqueue a channel open request (outbound) and return a future expecting the reply.
    ///
    /// The future resolves immediately with a `ConnectionError` in case the connection is dead.
    ///
    /// Otherwise, this flags the inner task for wakeup, but doesn't actually wake it up right away.
    /// Use `inner_task_waker` afterwards to obtain a `Waker` and use it after releasing the lock!
    pub fn open<C: Channel>(&mut self, o: C::Open) -> ChannelOpenFuture<C> {
        if let Some(ref result) = self.result {
            let (tx, rx) = oneshot::channel();
            tx.send(Err(result.clone().into()));
            return ChannelOpenFuture::new(rx);
        }

        let rx = self
            .channels
            .open_outbound(C::NAME, SshCodec::encode(&o).unwrap()); // FIXME FIXME !!!
        self.flag_inner_task_for_wakeup();
        ChannelOpenFuture::new(rx)
    }

    /// Set the result to `Err(TransportError::DisconnectByUs(reason).into())` (if not yet set).
    ///
    /// This flags the inner task for wakeup, but doesn't actually wake it up right away.
    /// Use `inner_task_waker` afterwards to obtain a `Waker` and use it after releasing the lock!
    pub fn disconnect(&mut self, reason: DisconnectReason) {
        let e = TransportError::DisconnectByUs(reason).into();
        let _ = self.result.get_or_insert(Err(e));
        self.flag_inner_task_for_wakeup();
    }

    /// Deliver a `ConnectionError` to all dependant users of this this connection (tasks waiting
    /// on connection requests or channel I/O).
    ///
    /// This shall be the last thing to be done by the `ConnectionFuture`.
    pub fn terminate(&mut self, result: Result<DisconnectReason, ConnectionError>) {
        let _ = self.result.get_or_insert(result.clone());
        let e: ConnectionError = result.into();
        while let Some(mut x) = self.local_requests.pop_front() {
            x.reply.take().map(|x| x.send(Err(e.clone()))).unwrap_or(())
        }
        while let Some(x) = self.local_replies.pop_front() {
            x.send(Err(e.clone()))
        }
        self.channels.terminate(e);
        self.flag_outer_task_for_wakeup();
    }

    /// Register the outer task to be notified new `ConnectionRequest`s.
    pub fn register_outer_task(&mut self, cx: &mut Context) {
        if let Some(ref waker) = self.outer_task_waker {
            if waker.will_wake(cx.waker()) {
                return;
            }
        }
        self.outer_task_waker = Some(cx.waker().clone());
    }

    /// Get a clone of the inner task's `Waker`.
    ///
    /// This returns `None` if the inner task is not registered or has not been flagged for wakeup.
    pub fn inner_task_waker(&mut self) -> Option<Waker> {
        if self.inner_task_wake {
            self.inner_task_wake = false;
            self.inner_task_waker.clone()
        } else {
            None
        }
    }

    /// Get a clone of the outer task's `Waker`.
    ///
    /// This returns `None` if the outer task is not registered or has not been flagged for wakeup.
    pub fn outer_task_waker(&mut self) -> Option<Waker> {
        if self.outer_task_wake {
            self.outer_task_wake = false;
            self.outer_task_waker.clone()
        } else {
            None
        }
    }

    /// Flag the inner task for wakepup (idempotent).
    pub fn flag_inner_task_for_wakeup(&mut self) {
        self.inner_task_wake = true;
    }

    /// Flag the inner task for wakepup (idempotent).
    pub fn flag_outer_task_for_wakeup(&mut self) {
        self.outer_task_wake = true;
    }

    /// Register the inner task to be notified on requests by an outer task.
    ///
    /// This may be called on each poll, but a `Waker` is only cloned once under the assumption
    /// that the `ConnectionFuture` is polled by the same task for the whole lifetime.
    fn register_inner_task(&mut self, cx: &mut Context) {
        if self.inner_task_waker.is_none() {
            self.inner_task_waker = Some(cx.waker().clone());
        }
    }

    /// Poll the connection result (ready when connection was terminated).
    /// Shall not be called again after readyness!
    ///
    /// If the result is a `TransportError::DisconnectByUs`, it is attempted to send a corresponding
    /// MSG_DISCONNECT to the peer (but we don't block if this fails or pends).
    fn poll_result(&mut self, cx: &mut Context) -> Poll<Result<DisconnectReason, ConnectionError>> {
        if let Some(ref r) = self.result {
            if let Err(ConnectionError::TransportError(TransportError::DisconnectByUs(d))) = r {
                self.transport.tx_disconnect(cx, *d);
            }
            return Poll::Ready(r.clone());
        }
        Poll::Pending
    }

    /// Poll the global outbound requests queue and try to send them in order.
    ///
    /// Return `Pending` as soon as the first request couldn't be sent on the transport.
    /// Eachs sucessfully transmitted request's reply gets the enqueued in the reply queue.
    fn poll_global_requests(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        while let Some(ref request) = self.local_requests.front() {
            let msg = MsgGlobalRequest::new(
                request.name.clone(),
                request.data.clone(),
                request.reply.is_some(),
            );
            ready!(poll_send(&mut self.transport, cx, &msg))?;
            if let Some(ref mut request) = self.local_requests.pop_front() {
                if let Some(reply) = request.reply.take() {
                    self.local_replies.push_back(reply);
                }
            }
        }
        Poll::Pending
    }

    /// Poll the global outbound replies queue and try to send them in order.
    ///
    /// Return `Pending` as soon as the first reply couldn't be sent on the transport.
    /// Each successfully transmitted reply gets removed from the reply queue.
    fn poll_global_replies(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        while let Some(ref mut future) = self.remote_replies.front_mut() {
            if let Some(data) = ready!(future.peek(cx)) {
                ready!(poll_send(
                    &mut self.transport,
                    cx,
                    &MsgRequestSuccess::new(&data?)
                ))?;
            } else {
                let msg = MsgRequestFailure;
                ready!(poll_send(&mut self.transport, cx, &msg))?;
            };
            let _ = self.remote_replies.pop_front();
        }
        Poll::Pending
    }

    /// Poll the transport for incoming messages.
    ///
    /// Firstly, it is attempted to flush the transport (eventually return with `Pending`).
    /// Secondly, it is attempted to receive from the transport and dispatch any incoming messages
    /// until `transport.poll_receive()` returns `Pending`.
    ///
    /// Only connection layer messages are considered. The dispatch order depends on the estimated
    /// likelyhood of message occurence (`MSG_CHANNEL_DATA` is the fastest path).
    ///
    /// In case a message cannot be decoded, it is tried to send a `MSG_UNIMPLEMENTED` to the client
    /// and the operations returns with a corresponding error.
    ///
    /// NB: Any message that is received gets dispatched. The dispatch mechanism does not cause
    /// the operation to return `Pending`. This is important to avoid deadlock situations!
    fn poll_transport(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        ready!(self.transport.tx_flush(cx))?;
        loop {
            let rx = ready!(self.transport.rx_peek(cx))?;
            // MSG_CHANNEL_DATA
            if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelData = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_DATA ({} bytes)",
                    msg.recipient_channel,
                    msg.data.len()
                );
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_data(msg.data)?;
                Ok(())
            }
            // MSG_CHANNEL_EXTENDED_DATA
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelExtendedData = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_EXTENDED_DATA ({} bytes)",
                    msg.recipient_channel,
                    msg.data.len()
                );
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_extended_data(msg.data_type_code, msg.data)?;
                Ok(())
            }
            // MSG_CHANNEL_WINDOW_ADJUST
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelWindowAdjust = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_WINDOW_ADJUST",
                    msg.recipient_channel
                );
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_window_adjust(msg.bytes_to_add)?;
                Ok(())
            }
            // MSG_CHANNEL_EOF
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelEof = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_EOF",
                    msg.recipient_channel
                );
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_eof()?;
                Ok(())
            }
            // MSG_CHANNEL_CLOSE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelClose = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_CLOSE",
                    msg.recipient_channel
                );
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_close()?;
                Ok(())
            }
            // MSG_CHANNEL_OPEN
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelOpen = msg;
                log::debug!("Received MSG_CHANNEL_OPEN ({})", msg.name);
                self.check_resource_exhaustion()?;
                self.channels.open_inbound(msg);
                self.flag_outer_task_for_wakeup();
                Ok(())
            }
            // MSG_CHANNEL_OPEN_CONFIRMATION
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelOpenConfirmation = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_OPEN_CONFIRMATION",
                    msg.recipient_channel
                );
                let channel = ChannelHandleInner::new(
                    msg.recipient_channel,
                    self.config.channel_max_buffer_size,
                    self.config.channel_max_packet_size,
                    msg.sender_channel,
                    msg.initial_window_size,
                    msg.maximum_packet_size,
                    false,
                );
                self.channels.accept(msg.recipient_channel, channel)?;
                Ok(())
            }
            // MSG_CHANNEL_OPEN_FAILURE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelOpenFailure = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_OPEN_FAILURE",
                    msg.recipient_channel
                );
                self.channels.reject(msg.recipient_channel, msg.reason)?;
                Ok(())
            }
            // MSG_CHANNEL_REQUEST
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelRequest<&[u8]> = msg;
                let rid = msg.recipient_channel;
                let req = msg.request.into();
                log::debug!("Channel {}: Received MSG_CHANNEL_REQUEST: {:?}", rid, req);
                self.check_resource_exhaustion()?;
                let channel = self.channels.get_open(rid)?;
                channel.push_request(req)?;
                Ok(())
            }
            // MSG_CHANNEL_SUCCESS
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelSuccess = msg;
                log::debug!(
                    "Channel {}: Received MSG_CHANNEL_SUCCESS",
                    msg.recipient_channel
                );
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_success()?;
                Ok(())
            }
            // MSG_CHANNEL_FAILURE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelFailure = msg;
                log::debug!("Received MSG_CHANNEL_FAILURE");
                let channel = self.channels.get_open(msg.recipient_channel)?;
                channel.push_failure()?;
                Ok(())
            }
            // MSG_GLOBAL_REQUEST
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgGlobalRequest = msg;
                log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
                self.check_resource_exhaustion()?;
                self.push_request(msg.name, msg.data, msg.want_reply)?;
                Ok(())
            }
            // MSG_REQUEST_SUCCESS
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgRequestSuccess = msg;
                log::debug!("Received MSG_REQUEST_SUCCESS");
                let data = msg.data.into();
                self.push_success(data)
            }
            // MSG_REQUEST_FAILURE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgRequestFailure = msg;
                log::debug!("Received MSG_REQUEST_FAILURE");
                self.push_failure()
            }
            // Otherwise try to send MSG_UNIMPLEMENTED and return error.
            else {
                // FIXME
                //self.transport.send_unimplemented(cx);
                Err(TransportError::InvalidState.into())
            }?;
            self.transport.rx_consume()?;
        }
    }

    fn push_request(
        &mut self,
        name: String,
        data: Vec<u8>,
        want_reply: bool,
    ) -> Result<(), ConnectionError> {
        let mut request = GlobalRequest {
            name,
            data,
            reply: None,
        };
        if want_reply {
            let (tx, rx) = oneshot::channel();
            request.reply = Some(tx);
            self.remote_replies.push_back(rx);
        }
        self.remote_requests.push_back(request);
        self.flag_outer_task_for_wakeup();
        Ok(())
    }

    fn push_success(&mut self, data: Vec<u8>) -> Result<(), ConnectionError> {
        if let Some(tx) = self.local_replies.pop_front() {
            tx.send(Ok(data));
            Ok(())
        } else {
            Err(ConnectionError::GlobalReplyUnexpected)
        }
    }

    fn push_failure(&mut self) -> Result<(), ConnectionError> {
        if let Some(tx) = self.local_replies.pop_front() {
            drop(tx);
            Ok(())
        } else {
            Err(ConnectionError::GlobalReplyUnexpected)
        }
    }

    fn queued(&self) -> usize {
        let mut c = self.channels.queued();
        c += self.remote_replies.len();
        c += self.remote_requests.len();
        c
    }

    fn check_resource_exhaustion(&self) -> Result<(), ConnectionError> {
        let exhausted = self.queued() >= self.config.queued_max_count as usize;
        check(!exhausted).ok_or(ConnectionError::ResourceExhaustion)
    }
}

pub fn poll_send<M: SshEncode>(
    t: &mut GenericTransport,
    cx: &mut Context,
    msg: &M,
) -> Poll<Result<(), TransportError>> {
    let size = SshCodec::size(msg).ok_or(TransportError::InvalidEncoding)?;
    let buf = ready!(t.tx_alloc(cx, size))?;
    SshCodec::encode_into(msg, buf).ok_or(TransportError::InvalidEncoding)?;
    t.tx_commit()?;
    Poll::Ready(Ok(()))
}

impl std::fmt::Debug for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectionState {{ ... }}")
    }
}
