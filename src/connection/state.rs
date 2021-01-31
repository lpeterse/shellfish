use super::channel::ChannelState;
use super::channel::*;
use super::request::*;
use super::*;

use crate::transport::{GenericTransport, TransportError};
use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;

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

pub struct ConnectionState {
    config: Arc<ConnectionConfig>,
    handler: Box<dyn ConnectionHandler>,
    transport: GenericTransport,
    requests_head: Option<ConnectionRequest>,
    requests_queue: mpsc::Receiver<ConnectionRequest>,
    requests_replies: VecDeque<oneshot::Sender<Result<Vec<u8>, ()>>>,
    replies_head: Option<Result<Vec<u8>, ()>>,
    replies_queue: VecDeque<oneshot::Receiver<Vec<u8>>>,
    channels: Vec<Option<Arc<Mutex<ChannelState>>>>,
    channels_rejections: VecDeque<u32>,
    close: oneshot::Sender<()>,
    error: watch::Sender<Option<ConnectionError>>,
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
            requests_replies: VecDeque::with_capacity(0),
            replies_head: None,
            replies_queue: VecDeque::with_capacity(0),
            channels: Vec::new(),
            channels_rejections: VecDeque::new(),
            close,
            error,
        }
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if let Poll::Ready(x) = self.poll_close(cx) {
            x?
        }
        if let Poll::Ready(x) = self.poll_transport(cx) {
            x?
        }
        if let Poll::Ready(x) = self.poll_channels_rejections(cx) {
            x?
        }
        if let Poll::Ready(x) = self.poll_replies(cx) {
            x?
        }
        if let Poll::Ready(x) = self.poll_requests(cx) {
            x?
        }
        // FIXME
        // if let Poll::Ready(x) = self.channels.poll(cx, &mut self.transport) {
        //     x?
        // }
        // The previous actions shall not actively flush the transport.
        // If necessary, the transport will be flushed here after all actions have eventually
        // written their output to the transport. This is benefecial for network performance as it
        // allows multiple messages to be sent in a single TCP segment (even with TCP_NODELAY) and
        // impedes traffic analysis.
        ready!(self.transport.tx_flush(cx))?;
        Poll::Pending
    }

    fn poll_close(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if self.close.poll_closed(cx).is_ready() || self.handler.poll(cx).is_ready() {
            let r = DisconnectReason::BY_APPLICATION;
            let e = match self.transport.tx_disconnect(cx, r) {
                Poll::Ready(Err(e)) => e,
                _ => TransportError::DisconnectByUs(r),
            };
            Poll::Ready(Err(e.into()))
        } else {
            Poll::Pending
        }
    }

    fn poll_channels_rejections(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        while let Some(rid) = self.channels_rejections.front() {
            let msg = MsgChannelOpenFailure::new(*rid, ChannelOpenFailure::RESOURCE_SHORTAGE);
            ready!(poll_send(&mut self.transport, cx, &msg))?;
            let _ = self.channels_rejections.pop_front();
        }
        Poll::Ready(Ok(()))
    }

    fn poll_replies(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if let Some(reply) = &self.replies_head {
            match reply {
                Ok(data) => {
                    let msg = MsgRequestSuccess { data: &data };
                    ready!(poll_send(&mut self.transport, cx, &msg))?;
                }
                Err(_) => {
                    let msg = MsgRequestFailure;
                    ready!(poll_send(&mut self.transport, cx, &msg))?;
                }
            }
            self.replies_head = None;
        }

        // Try to send ready replies in original order and remove those sent
        while let Some(mut x) = self.replies_queue.front_mut() {
            let reply = ready!(Future::poll(Pin::new(&mut x), cx));
            let _ = self.replies_queue.pop_front();
            match reply {
                Ok(data) => {
                    let msg = MsgRequestSuccess { data: &data };
                    if poll_send(&mut self.transport, cx, &msg)?.is_pending() {
                        self.replies_head = Some(Ok(data));
                        return Poll::Pending;
                    }
                }
                Err(_) => {
                    let msg = MsgRequestFailure;
                    if poll_send(&mut self.transport, cx, &msg)?.is_pending() {
                        self.replies_head = Some(Err(()));
                        return Poll::Pending;
                    }
                }
            }
        }

        // Save memory by shrinking vectors to fit
        self.replies_queue.shrink_to_fit();
        Poll::Ready(Ok(()))
    }

    fn poll_requests(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        loop {
            let cr = if let Some(cr) = self.requests_head.take() {
                cr
            } else {
                ready!(self.requests_queue.poll_recv(cx)).ok_or(ConnectionError::Dropped)?
            };
            match cr {
                ConnectionRequest::Global { name, data, reply } => {
                    let msg = MsgGlobalRequest {
                        name: &name,
                        data: &data.as_ref(),
                        want_reply: reply.is_some(),
                    };
                    match poll_send(&mut self.transport, cx, &msg) {
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
                        match poll_send(&mut self.transport, cx, &msg) {
                            Poll::Ready(r) => r?,
                            Poll::Pending => {
                                let cr = ConnectionRequest::Open { name, data, reply };
                                self.requests_head = Some(cr);
                                return Poll::Pending;
                            }
                        }
                        let cst = ChannelState::new_outbound(lid, lws, lps, reply);
                        self.channels[lid as usize] = Some(Arc::new(Mutex::new(cst)));
                    } else {
                        let e = ChannelOpenFailure::RESOURCE_SHORTAGE;
                        reply.send(Err(e)).unwrap_or(());
                    }
                }
            }
        }
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
            // MSG_CHANNEL_OPEN
            if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelOpen = msg;
                log::debug!("Received MSG_CHANNEL_OPEN ({})", msg.name);
                if let Some(lid) = self.alloc_channel_id() {
                    let (s, r) = oneshot::channel();
                    let lws = self.config.channel_max_buffer_size;
                    let lps = self.config.channel_max_packet_size;
                    let rid = msg.sender_channel;
                    let rws = msg.initial_window_size;
                    let rps = msg.maximum_packet_size;
                    let cst = ChannelState::new_inbound(lid, lws, lps, rid, rws, rps, false, r);
                    let cst = Arc::new(Mutex::new(cst));
                    let req = ChannelOpenRequest {
                        name: msg.name,
                        data: msg.data,
                        chan: ChannelHandle(cst.clone()),
                        resp: s,
                    };
                    self.channels[lid as usize] = Some(cst);
                    self.handler.on_open_request(req)
                } else {
                    self.channels_rejections.push_back(msg.sender_channel);
                }
                Ok(())
            }
            // MSG_CHANNEL_OPEN_CONFIRMATION
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelOpenConfirmation = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_OPEN_CONFIRMATION", lid);
                channel!(self, lid).accept2()?;
                Ok(())
            }
            // MSG_CHANNEL_OPEN_FAILURE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelOpenFailure = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_OPEN_FAILURE", lid);
                channel!(self, lid).reject2(msg.reason)?;
                Ok(())
            }
            // MSG_CHANNEL_DATA
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelData = msg;
                let lid = msg.recipient_channel;
                let len = msg.data.len();
                log::debug!("Channel {}: Received MSG_CHANNEL_DATA ({} bytes)", lid, len);
                channel!(self, lid).push_data(msg.data)?;
                Ok(())
            }
            // MSG_CHANNEL_EXTENDED_DATA
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelExtendedData = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_EXTENDED_DATA", lid);
                channel!(self, lid).push_extended_data(msg.data_type_code, msg.data)?;
                Ok(())
            }
            // MSG_CHANNEL_WINDOW_ADJUST
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelWindowAdjust = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_WINDOW_ADJUST", lid);
                channel!(self, lid).push_window_adjust(msg.bytes_to_add)?;
                Ok(())
            }
            // MSG_CHANNEL_EOF
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelEof = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_EOF", lid);
                channel!(self, lid).push_eof()?;
                Ok(())
            }
            // MSG_CHANNEL_CLOSE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelClose = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_CLOSE", lid);
                channel!(self, lid).push_close()?;
                Ok(())
            }
            // MSG_CHANNEL_REQUEST
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelRequest<&[u8]> = msg;
                let lid = msg.recipient_channel;
                let typ = msg.request;
                log::debug!("Channel {}: Received MSG_CHANNEL_REQUEST: {:?}", lid, typ);
                channel!(self, lid).push_request(typ, msg.specific, msg.want_reply)?;
                Ok(())
            }
            // MSG_CHANNEL_SUCCESS
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelSuccess = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_SUCCESS", lid);
                channel!(self, lid).push_success()?;
                Ok(())
            }
            // MSG_CHANNEL_FAILURE
            else if let Some(msg) = SshCodec::decode(rx) {
                let _: MsgChannelFailure = msg;
                let lid = msg.recipient_channel;
                log::debug!("Channel {}: Received MSG_CHANNEL_FAILURE", lid);
                channel!(self, lid).push_failure()?;
                Ok(())
            }
            // MSG_GLOBAL_REQUEST
            else if let Some(msg) = SshCodec::decode(rx) {
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

    fn push_success(&mut self, data: Vec<u8>) -> Result<(), ConnectionError> {
        if let Some(sender) = self.requests_replies.pop_front() {
            let _ = sender.send(Ok(data));
            Ok(())
        } else {
            Err(ConnectionError::GlobalReplyUnexpected)
        }
    }

    fn push_failure(&mut self) -> Result<(), ConnectionError> {
        if let Some(sender) = self.requests_replies.pop_front() {
            let _ = sender.send(Err(()));
            Ok(())
        } else {
            Err(ConnectionError::GlobalReplyUnexpected)
        }
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

    /// Deliver a `ConnectionError` to all dependant users of this this connection (tasks waiting
    /// on connection requests or channel I/O).
    ///
    /// This shall be the last thing to be done by the `ConnectionFuture`.
    fn terminate(&mut self, e: ConnectionError) {
        std::mem::replace(&mut self.handler, Box::new(())).on_error(&e);
        // FIXME
        //self.channels.terminate(e.clone());
        self.error.send(Some(e)).unwrap_or(());
    }
}

impl Future for ConnectionState {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let self_ = Pin::into_inner(self);
        if let Err(e) = ready!(self_.poll(cx)) {
            self_.terminate(e);
        }
        log::error!("CONNECTION DROP");
        Poll::Ready(())
    }
}

// FIXME: Move to transport?
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
