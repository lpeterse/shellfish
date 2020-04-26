use super::channel::*;
use super::*;

use crate::transport::{DisconnectReason, TransportError};
use crate::util::assume;

use async_std::task::{ready, Context, Poll, Waker};
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
#[derive(Debug)]
pub struct ConnectionState<T: TransportLayer = Transport> {
    config: Arc<ConnectionConfig>,
    transport: T,
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

impl<T: TransportLayer> ConnectionState<T> {
    /// Create a new state with config and transport.
    ///
    /// The config `Arc` will be cloned. All queues are initialised with zero
    /// capacity in expectation that they will never be used.
    pub fn new(config: &Arc<ConnectionConfig>, transport: T) -> Self {
        Self {
            config: config.clone(),
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
        ready!(self.transport.poll_flush(cx))?;
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
        self.result
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
        if let Some(result) = self.result {
            let (tx, rx) = oneshot::channel();
            tx.send(Err(result.into()));
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
        if let Some(result) = self.result {
            let (tx, rx) = oneshot::channel();
            tx.send(Err(result.into()));
            return ChannelOpenFuture::new(rx);
        }

        let rx = self.channels.open_outbound(C::NAME, BEncoder::encode(&o));
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
        let _ = self.result.get_or_insert(result);
        let e = result.into();
        while let Some(mut x) = self.local_requests.pop_front() {
            x.reply.take().map(|x| x.send(Err(e))).unwrap_or(())
        }
        while let Some(x) = self.local_replies.pop_front() {
            x.send(Err(e))
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
    fn flag_inner_task_for_wakeup(&mut self) {
        self.inner_task_wake = true;
    }

    /// Flag the inner task for wakepup (idempotent).
    fn flag_outer_task_for_wakeup(&mut self) {
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
        if let Some(r) = self.result {
            if let Err(ConnectionError::TransportError(TransportError::DisconnectByUs(d))) = r {
                self.transport.send_disconnect(cx, d);
            }
            return Poll::Ready(r);
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
            ready!(self.transport.poll_send(cx, &msg))?;
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
                ready!(self
                    .transport
                    .poll_send(cx, &MsgRequestSuccess::new(&data?)))?;
            } else {
                ready!(self.transport.poll_send(cx, &MsgRequestFailure))?;
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
    /// NB: Any message that is received gets dispatched. The dispatch mechanism does not cause
    /// the operation to return `Pending`. This is important to avoid deadlock situations!
    fn poll_transport(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        ready!(self.transport.poll_flush(cx))?;
        loop {
            ready!(self.transport.poll_receive(cx))?;
            self.dispatch_transport(cx)?;
            self.transport.consume();
        }
    }

    /// Try to decode, dispatch and consume the next inbound message.
    ///
    /// Only connection layer messages are considered. The dispatch order depends on the estimated
    /// likelyhood of message occurence (`MSG_CHANNEL_DATA` is the fastest path).
    ///
    /// In case a message cannot be decoded, it is tried to send a `MSG_UNIMPLEMENTED` to the client
    /// and the operations returns with a corresponding error.
    fn dispatch_transport(&mut self, cx: &mut Context) -> Result<(), ConnectionError> {
        // MSG_CHANNEL_DATA
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelData = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_DATA ({} bytes)",
                msg.recipient_channel,
                msg.data.len()
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_data(msg.data)?;
            return Ok(());
        }
        // MSG_CHANNEL_EXTENDED_DATA
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelExtendedData = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_EXTENDED_DATA ({} bytes)",
                msg.recipient_channel,
                msg.data.len()
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_extended_data(msg.data_type_code, msg.data)?;
            return Ok(());
        }
        // MSG_CHANNEL_WINDOW_ADJUST
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelWindowAdjust = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_WINDOW_ADJUST",
                msg.recipient_channel
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_window_adjust(msg.bytes_to_add)?;
            return Ok(());
        }
        // MSG_CHANNEL_EOF
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelEof = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_EOF",
                msg.recipient_channel
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_eof()?;
            return Ok(());
        }
        // MSG_CHANNEL_CLOSE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelClose = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_CLOSE",
                msg.recipient_channel
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_close()?;
            return Ok(());
        }
        // MSG_CHANNEL_OPEN
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelOpen = msg;
            log::debug!("Received MSG_CHANNEL_OPEN ({})", msg.name);
            self.check_resource_exhaustion()?;
            self.channels.open_inbound(msg);
            self.flag_outer_task_for_wakeup();
            return Ok(());
        }
        // MSG_CHANNEL_OPEN_CONFIRMATION
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelOpenConfirmation = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_OPEN_CONFIRMATION",
                msg.recipient_channel
            );
            let channel = ChannelState::new(
                msg.recipient_channel,
                self.config.channel_max_window_size,
                self.config.channel_max_packet_size,
                msg.sender_channel,
                msg.initial_window_size,
                msg.maximum_packet_size,
                false,
            );
            self.channels.accept(msg.recipient_channel, channel)?;
            return Ok(());
        }
        // MSG_CHANNEL_OPEN_FAILURE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelOpenFailure = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_OPEN_FAILURE",
                msg.recipient_channel
            );
            self.channels.reject(msg.recipient_channel, msg.reason)?;
            return Ok(());
        }
        // MSG_CHANNEL_REQUEST
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelRequest<&[u8]> = msg;
            let rid = msg.recipient_channel;
            let req = msg.request.into();
            log::debug!("Channel {}: Received MSG_CHANNEL_REQUEST: {:?}", rid, req);
            self.check_resource_exhaustion()?;
            let channel = self.channels.get_open(rid)?;
            channel.push_request(req)?;
            return Ok(());
        }
        // MSG_CHANNEL_SUCCESS
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelSuccess = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_SUCCESS",
                msg.recipient_channel
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_success()?;
            return Ok(());
        }
        // MSG_CHANNEL_FAILURE
        if let Some(msg) = self.transport.decode() {
            let _: MsgChannelFailure = msg;
            log::debug!("Received MSG_CHANNEL_FAILURE");
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_failure()?;
            return Ok(());
        }
        // MSG_GLOBAL_REQUEST
        if let Some(msg) = self.transport.decode() {
            let _: MsgGlobalRequest = msg;
            log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
            self.check_resource_exhaustion()?;
            self.push_request(msg.name, msg.data, msg.want_reply)?;
            return Ok(());
        }
        // MSG_REQUEST_SUCCESS
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgRequestSuccess = msg;
            log::debug!("Received MSG_REQUEST_SUCCESS");
            let data = msg.data.into();
            return self.push_success(data);
        }
        // MSG_REQUEST_FAILURE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgRequestFailure = msg;
            log::debug!("Received MSG_REQUEST_FAILURE");
            return self.push_failure();
        }
        // Otherwise try to send MSG_UNIMPLEMENTED and return error.
        self.transport.send_unimplemented(cx);
        Err(TransportError::MessageUnexpected.into())
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
        assume(!exhausted).ok_or(ConnectionError::ResourceExhaustion)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::transport::tests::TestTransport;

    /// Test internal state after creation.
    #[test]
    fn test_connection_state_new_01() {
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let x = ConnectionState::new(&c, t);

        assert_eq!(x.result.is_some(), false);
        assert_eq!(x.inner_task_wake, false);
        assert_eq!(x.outer_task_wake, false);
    }

    /// Test poll after creation.
    #[test]
    fn test_connection_state_poll_01() {
        use async_std::future::poll_fn;
        use async_std::task::*;

        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx).is_pending(), true);
            assert_eq!(x.inner_task_wake, false);
            assert_eq!(x.outer_task_wake, false);
            assert_eq!(x.transport.send_count(), 0, "send_count");
            assert_eq!(x.transport.receive_count(), 1, "receive_count");
            assert_eq!(x.transport.consume_count(), 0, "consume_count");
            assert_eq!(x.transport.flush_count(), 2, "flush_count");
            Poll::Ready(())
        }));
    }

    /// Test poll after disconnect.
    #[test]
    fn test_connection_state_poll_02() {
        use async_std::future::poll_fn;
        use async_std::task::*;

        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        let reason = DisconnectReason::BY_APPLICATION;
        x.transport.set_tx_ready(true);
        x.disconnect(reason);

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx).is_pending(), false);
            assert_eq!(x.transport.tx_disconnect(), Some(reason));
            Poll::Ready(())
        }));
    }

    /// Test poll with ready global reply (transport ready).
    #[test]
    fn test_connection_state_poll_03() {
        use async_std::future::poll_fn;
        use async_std::task::*;

        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        x.transport.set_tx_ready(true);
        // Accept
        let (tx, rx) = oneshot::channel();
        tx.send(Ok(b"abc"[..].into()));
        x.remote_replies.push_back(rx);
        // Reject
        let (_, rx) = oneshot::channel();
        x.remote_replies.push_back(rx);

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx).is_pending(), true);
            assert_eq!(x.outer_task_wake, false);
            assert_eq!(x.inner_task_wake, false);
            assert_eq!(
                x.transport.tx_sent(),
                vec![vec![vec![81, 97, 98, 99], vec![82]]]
            );
            Poll::Ready(())
        }));
    }

    /// Test poll with ready global request (transport ready).
    #[test]
    fn test_connection_state_poll_04() {
        use async_std::future::poll_fn;
        use async_std::task::*;

        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        x.transport.set_tx_ready(true);
        let req = GlobalRequest::new("abc".into(), vec![1, 2, 3]);
        x.local_requests.push_back(req);

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx).is_pending(), true);
            assert_eq!(x.inner_task_wake, false);
            assert_eq!(x.outer_task_wake, false);
            assert_eq!(
                x.transport.tx_sent(),
                vec![vec![vec![80, 0, 0, 0, 3, 97, 98, 99, 0, 1, 2, 3]]]
            );
            Poll::Ready(())
        }));
    }

    /// Test poll with ready channel open request (transport ready).
    #[test]
    fn test_connection_state_poll_05() {
        use async_std::future::poll_fn;
        use async_std::task::*;
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);
        x.transport.set_tx_ready(true);

        let _ = x.open::<Session<Client>>(());

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx).is_pending(), true);
            assert_eq!(x.inner_task_wake, true);
            assert_eq!(x.outer_task_wake, false);
            assert_eq!(
                x.transport.tx_sent(),
                vec![vec![vec![
                    90, 0, 0, 0, 7, 115, 101, 115, 115, 105, 111, 110, 0, 0, 0, 0, 0, 16, 0, 0, 0,
                    0, 128, 0
                ]]]
            );
            Poll::Ready(())
        }));
    }

    /// Test poll with global inbound request ready on transport.
    #[test]
    fn test_connection_state_poll_06() {
        use async_std::future::poll_fn;
        use async_std::task::*;

        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        x.transport.set_tx_ready(true);
        x.transport
            .rx_push(&MsgGlobalRequest::new("abc", vec![], false));

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx).is_pending(), true);
            assert_eq!(x.inner_task_wake, false);
            assert_eq!(x.outer_task_wake, true);
            assert_eq!(x.remote_requests.len(), 1);
            Poll::Ready(())
        }));
    }

    /// Test next after creation.
    #[test]
    fn test_connection_state_next_01() {
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        assert_eq!(x.next().is_none(), true);
        assert_eq!(x.inner_task_wake, false);
        assert_eq!(x.outer_task_wake, false);
    }

    /// Test next when global request present.
    #[test]
    fn test_connection_state_next_02() {
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        let req = GlobalRequest::new("abc".into(), vec![1, 2, 3]);
        x.remote_requests.push_back(req);

        match x.next() {
            Some(ConnectionRequest::Global(_)) => (),
            _ => panic!("expected global request"),
        }
        assert_eq!(x.inner_task_wake, true);
        assert_eq!(x.outer_task_wake, false);
    }

    /// Test next when channel open request present.
    #[test]
    fn test_connection_state_next_03() {
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        let msg = MsgChannelOpen::new("session".into(), 0, 0, 0, vec![]);
        x.channels.open_inbound(msg);

        match x.next() {
            Some(ConnectionRequest::ChannelOpen(_)) => (),
            _ => panic!("expected channel open request"),
        }
        assert_eq!(x.inner_task_wake, true);
        assert_eq!(x.outer_task_wake, false);
    }

    /// Test request_global.
    #[test]
    fn test_connection_state_request_global_01() {
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        assert_eq!(x.local_requests.len(), 0);
        x.request("abc".into(), vec![123]);
        assert_eq!(x.local_requests.len(), 1);
    }

    /// Test request_global_want_reply.
    #[test]
    fn test_connection_state_request_global_want_reply_01() {
        let c = Arc::new(ConnectionConfig::default());
        let t = TestTransport::new();
        let mut x = ConnectionState::new(&c, t);

        assert_eq!(x.local_requests.len(), 0);
        x.request_want_reply("abc".into(), vec![123]);
        assert_eq!(x.local_requests.len(), 1);
    }
}
