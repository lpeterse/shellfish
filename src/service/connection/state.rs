use super::channel::*;
use super::*;

use crate::transport::{DisconnectReason, TransportError};

use async_std::task::{ready, Context, Poll, Waker};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct ConnectionState<T: TransportLayer> {
    config: Arc<ConnectionConfig>,
    transport: T,
    error: Result<(), ConnectionError>,
    disconnect: Option<DisconnectReason>,
    channels: ChannelSlots,
    global_in_requests: VecDeque<GlobalRequest>,
    global_in_replies: VecDeque<oneshot::Receiver<Result<Vec<u8>, ConnectionError>>>,
    global_out_requests: VecDeque<GlobalRequest>,
    global_out_replies: VecDeque<oneshot::Sender<Result<Vec<u8>, ConnectionError>>>,
    future_task: Option<Waker>,
    handle_task: Option<Waker>,
}

#[derive(Debug)]
pub enum ConnectionRequest {
    Global(GlobalRequest),
    ChannelOpen(ChannelOpenRequest),
}

impl<T: TransportLayer> ConnectionState<T> {
    pub fn new(config: &Arc<ConnectionConfig>, transport: T) -> Self {
        Self {
            config: config.clone(),
            transport,
            error: Ok(()),
            disconnect: None,
            channels: ChannelSlots::new(&config),
            global_in_requests: VecDeque::with_capacity(1),
            global_in_replies: VecDeque::with_capacity(1),
            global_out_requests: VecDeque::with_capacity(1),
            global_out_replies: VecDeque::with_capacity(1),
            future_task: None,
            handle_task: None,
        }
    }

    pub fn poll(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        self.register_future_task(cx);
        // Check disconnect flag (returns error if set)
        if let Poll::Ready(x) = self.poll_disconnect(cx) {
            x?
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
        if let Poll::Ready(x) = self.poll_channels(cx) {
            x?
        }
        // The 3 previous actions shall not actively flush the transport.
        // If necessary, the transport will be flushed here after all actions have eventually
        // written their output to the transport. This is benefecial for network performance
        // as it allows multiple messages to be sent in a single TCP segment (even with
        // TCP_NODELAY) and impedes traffic analysis.
        ready!(self.transport.poll_flush(cx))?;
        Poll::Pending
    }

    pub fn poll_next(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<ConnectionRequest, ConnectionError>> {
        self.error?;
        if let Some(request) = self.global_in_requests.pop_front() {
            self.handle_task = None;
            self.wake_future_task();
            return Poll::Ready(Ok(ConnectionRequest::Global(request)));
        }
        if let Some(request) = self.channels.take_open_request() {
            self.handle_task = None;
            return Poll::Ready(Ok(ConnectionRequest::ChannelOpen(request)));
        }
        self.register_handle_task(cx);
        Poll::Pending
    }

    pub fn request_global(&mut self, name: String, data: Vec<u8>) {
        let request = GlobalRequest::new(name, data);
        self.global_out_requests.push_back(request);
        self.wake_future_task();
    }

    pub fn request_global_want_reply(&mut self, name: String, data: Vec<u8>) -> ReplyFuture {
        let (request, reply) = GlobalRequest::new_want_reply(name, data);
        self.global_out_requests.push_back(request);
        self.wake_future_task();
        reply
    }

    pub fn open_channel<C: Channel>(&mut self, params: C::Open) -> ChannelOpenFuture<C> {
        let rx = if let Err(e) = self.error {
            let (tx, rx) = oneshot::channel();
            tx.send(Err(e));
            rx
        } else if let Some(rx) = self
            .channels
            .open_outbound(C::NAME, BEncoder::encode(&params))
        {
            self.wake_future_task();
            rx
        } else {
            let (tx, rx) = oneshot::channel();
            tx.send(Ok(Err(ChannelOpenFailureReason::RESOURCE_SHORTAGE)));
            rx
        };
        ChannelOpenFuture::new(rx)
    }

    pub fn disconnect(&mut self, reason: DisconnectReason) {
        self.disconnect = Some(reason);
        self.wake_future_task();
    }

    fn wake_future_task(&mut self) {
        if let Some(ref task) = self.future_task {
            task.wake_by_ref();
        }
    }

    fn register_handle_task(&mut self, cx: &mut Context) {
        if let Some(ref waker) = self.handle_task {
            if waker.will_wake(cx.waker()) {
                return;
            }
        }
        self.handle_task = Some(cx.waker().clone());
    }

    pub fn wake_handle_task(&mut self) {
        if let Some(ref task) = self.handle_task {
            task.wake_by_ref();
        }
    }

    fn register_future_task(&mut self, cx: &mut Context) {
        if self.future_task.is_none() {
            self.future_task = Some(cx.waker().clone());
        }
    }

    fn poll_disconnect(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if let Some(reason) = self.disconnect {
            self.transport.send_disconnect(cx, reason);
            return Poll::Ready(Err(TransportError::DisconnectByUs(reason).into()));
        }
        Poll::Pending
    }

    fn poll_channels(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        // Iterate over all channel slots and poll each present channel.
        // Remove channel if the futures is ready (close has been sent _and_ received).
        for (id, slot) in self.channels.iter_mut().enumerate() {
            'inner: loop {
                match slot {
                    ChannelSlot::Free => (),
                    ChannelSlot::OpeningInbound1(_) => (),
                    ChannelSlot::OpeningInbound2(x) => {
                        let e = Err(ChannelOpenFailureReason::ADMINISTRATIVELY_PROHIBITED);
                        match x.rx.peek(cx).map(|x| x.unwrap_or(e)) {
                            Poll::Ready(Ok(())) => {
                                let msg = MsgChannelOpenConfirmation {
                                    recipient_channel: x.rid,
                                    sender_channel: id as u32,
                                    initial_window_size: self.config.channel_max_window_size,
                                    maximum_packet_size: self.config.channel_max_packet_size,
                                    specific: &[],
                                };
                                ready!(self.transport.poll_send(cx, &msg))?;
                                let y = std::mem::replace(slot, ChannelSlot::Free);
                                if let ChannelSlot::OpeningInbound2(y) = y {
                                    *slot = ChannelSlot::Open(y.ch);
                                }
                                continue 'inner;
                            }
                            Poll::Ready(Err(reason)) => {
                                let msg = MsgChannelOpenFailure::new(x.rid, reason);
                                ready!(self.transport.poll_send(cx, &msg))?;
                                *slot = ChannelSlot::Free;
                            }
                            Poll::Pending => (),
                        }
                    }
                    ChannelSlot::OpeningOutbound(x) => {
                        if !x.sent {
                            let msg = MsgChannelOpen::new(
                                x.name,
                                id as u32,
                                self.config.channel_max_window_size as u32,
                                self.config.channel_max_packet_size as u32,
                                x.data.clone(),
                            );
                            ready!(self.transport.poll_send(cx, &msg))?;
                            x.sent = true;
                        }
                    }
                    ChannelSlot::Open(channel) => match channel.poll(cx, &mut self.transport) {
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok(())) => *slot = ChannelSlot::Free,
                        Poll::Pending => (),
                    },
                }
                break 'inner;
            }
        }
        Poll::Pending
    }

    fn poll_global_requests(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        while let Some(ref request) = self.global_out_requests.front() {
            let msg = MsgGlobalRequest {
                name: request.name.clone(),
                data: request.data.clone(),
                want_reply: request.reply.is_some(),
            };
            ready!(self.transport.poll_send(cx, &msg))?;
            if let Some(ref mut request) = self.global_out_requests.pop_front() {
                if let Some(reply) = request.reply.take() {
                    self.global_out_replies.push_back(reply);
                }
            }
        }
        Poll::Pending
    }

    /// Try all pending replies in the correct order (sic!).
    /// Stop on the first one that is not ready.
    fn poll_global_replies(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        while let Some(ref mut future) = self.global_in_replies.front_mut() {
            if let Some(data) = ready!(future.peek(cx)) {
                ready!(self
                    .transport
                    .poll_send(cx, &MsgRequestSuccess::new(&data?)))?;
            } else {
                ready!(self.transport.poll_send(cx, &MsgRequestFailure))?;
            };
            let _ = self.global_in_replies.pop_front();
            self.wake_handle_task();
        }
        Poll::Pending
    }

    fn poll_transport(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        ready!(self.transport.poll_flush(cx))?;
        loop {
            ready!(self.transport.poll_receive(cx))?;
            self.dispatch_transport(cx)?;
            self.transport.consume();
        }
    }

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
            self.channels.open_inbound(msg);
            self.wake_handle_task();
            return Ok(());
        }
        // MSG_CHANNEL_OPEN_CONFIRMATION
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgChannelOpenConfirmation = msg;
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_OPEN_CONFIRMATION",
                msg.recipient_channel
            );
            let channel = ChannelHandle::new(
                msg.recipient_channel,
                self.config.channel_max_window_size,
                self.config.channel_max_packet_size,
                msg.sender_channel,
                msg.initial_window_size,
                msg.maximum_packet_size,
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
            log::debug!(
                "Channel {}: Received MSG_CHANNEL_REQUEST: {}",
                msg.recipient_channel,
                msg.request
            );
            let channel = self.channels.get_open(msg.recipient_channel)?;
            channel.push_request(msg.specific)?;
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
            self.push_global_request(cx, msg.name, msg.data, msg.want_reply)?;
            return Ok(());
        }
        // MSG_REQUEST_SUCCESS
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgRequestSuccess = msg;
            log::debug!("Received MSG_REQUEST_SUCCESS");
            if let Some(tx) = self.global_out_replies.pop_front() {
                tx.send(Ok(msg.data.into()));
            } else {
                return Err(ConnectionError::GlobalRequestReplyUnexpected);
            }
            return Ok(());
        }
        // MSG_REQUEST_FAILURE
        if let Some(msg) = self.transport.decode_ref() {
            let _: MsgRequestFailure = msg;
            log::debug!("Received MSG_REQUEST_FAILURE");
            if let Some(tx) = self.global_out_replies.pop_front() {
                drop(tx);
            } else {
                return Err(ConnectionError::GlobalRequestReplyUnexpected);
            }
            return Ok(());
        }
        // Otherwise try to send MSG_UNIMPLEMENTED and return error.
        self.transport.send_unimplemented(cx);
        Err(TransportError::MessageUnexpected.into())
    }

    fn push_global_request(
        &mut self,
        _cx: &mut Context,
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
            self.global_in_replies.push_back(rx);
        }
        self.global_in_requests.push_back(request);
        self.wake_handle_task();
        Ok(())
    }
}

impl<T: TransportLayer> Terminate for ConnectionState<T> {
    /// Deliver a `ConnectionError` to all dependant users of this this connection (tasks waiting
    /// on connection requests or channel I/O).
    ///
    /// This shall be the last thing to happen and has great similarity with `Drop` except that
    /// it distributes an error.
    fn terminate(&mut self, e: ConnectionError) {
        while let Some(mut x) = self.global_out_requests.pop_front() {
            x.reply.take().map(|x| x.send(Err(e))).unwrap_or(())
        }
        while let Some(x) = self.global_out_replies.pop_front() {
            x.send(Err(e))
        }
        self.channels.terminate(e);
        self.error = Err(e);
        self.wake_handle_task();
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

        assert!(x.error.is_ok());
        assert!(x.disconnect.is_none());
        assert!(x.future_task.is_none());
        assert!(x.handle_task.is_none());
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
            assert_eq!(x.poll(cx), Poll::Pending, "poll");
            assert_eq!(x.transport.send_count(), 0, "send_count");
            assert_eq!(x.transport.receive_count(), 1, "receive_count");
            assert_eq!(x.transport.consume_count(), 0, "consume_count");
            assert_eq!(x.transport.flush_count(), 2, "flush_count");
            assert!(x.future_task.is_some(), "future_task");
            assert!(x.handle_task.is_none(), "handle_task");
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

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx), Poll::Pending, "poll");
            assert_eq!(x.transport.send_count(), 0, "send_count");
            assert_eq!(x.transport.receive_count(), 1, "receive_count");
            assert_eq!(x.transport.consume_count(), 0, "consume_count");
            assert_eq!(x.transport.flush_count(), 2, "flush_count");
            assert!(x.future_task.is_some(), "future_task");
            assert!(x.handle_task.is_none(), "handle_task");
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
        x.global_in_replies.push_back(rx);

        // Reject
        let (_, rx) = oneshot::channel();
        x.global_in_replies.push_back(rx);

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx), Poll::Pending, "poll");
            assert!(x.future_task.is_some(), "future_task");
            assert!(x.handle_task.is_none(), "handle_task");
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
        x.global_out_requests.push_back(req);

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx), Poll::Pending, "poll");
            assert!(x.future_task.is_some(), "future_task");
            assert!(x.handle_task.is_none(), "handle_task");
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
        
        #[allow(unused_variables)]
        let _ = x.open_channel::<Session<Client>>(());

        block_on(poll_fn(|cx| {
            assert_eq!(x.poll(cx), Poll::Pending, "poll");
            assert!(x.future_task.is_some(), "future_task");
            assert!(x.handle_task.is_none(), "handle_task");
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
}
