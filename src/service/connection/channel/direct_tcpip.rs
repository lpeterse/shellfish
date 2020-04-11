mod open;

use super::*;

pub(crate) use self::open::*;

use crate::buffer::*;

use async_std::io::Read;
use async_std::task::Poll;
use async_std::task::Waker;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct DirectTcpIp(pub(crate) ChannelState42);

#[derive(Clone, Debug)]
pub struct ChannelState42(Arc<Mutex<ChannelState43>>);

#[derive(Debug)]
pub enum R {
    A(oneshot::Sender<Result<ChannelState42, ChannelOpenFailureReason>>),
    B {
        id: u32,
        window_size: u32,
        max_packet_size: u32,
    },
}

impl R {
    pub fn new(id: u32, ws: u32, ps: u32) -> Self {
        Self::B {
            id,
            window_size: ws,
            max_packet_size: ps,
        }
    }
}

impl ChannelOpen for DirectTcpIp {
    type Open = DirectTcpIpOpen;
    type Confirmation = ();
}

impl Channel for DirectTcpIp {
    type Request = ();
    //type State = ChannelState43;

    const NAME: &'static str = "direct-tcpip";
}

impl ChannelState42 {
    pub fn new_state(
        max_window_size: u32,
        max_packet_size: u32,
        reply: oneshot::Sender<Result<Self, ChannelOpenFailureReason>>,
    ) -> Self {
        Self(Arc::new(Mutex::new(ChannelState43 {
            local_window_size: max_window_size,
            local_max_window_size: max_window_size,
            local_max_packet_size: max_packet_size,

            remote: R::A(reply),

            is_eof_sent: false,
            is_eof_received: false,
            is_close_sent: false,
            is_close_received: false,

            inner_task: None,
            inner_done: None,
            outer_task: None,
            outer_done: None,

            data_in: Buffer::new(max_window_size as usize),
            data_out: Buffer::new(max_window_size as usize),
        })))
    }
}

#[derive(Debug)]
pub(crate) struct ChannelState43 {
    local_window_size: u32,
    local_max_window_size: u32,
    local_max_packet_size: u32,

    remote: R,

    is_eof_sent: bool,
    is_eof_received: bool,
    is_close_sent: bool,
    is_close_received: bool,

    inner_task: Option<Waker>,
    inner_done: Option<ConnectionError>,
    outer_task: Option<Waker>,
    outer_done: Option<()>,

    data_in: Buffer,
    data_out: Buffer,
}

impl ChannelState42 {
    pub fn push_open_confirmation(
        &mut self,
        ch: u32,
        ws: u32,
        ps: u32,
    ) -> Result<(), ConnectionError> {
        let self_ = self.clone();
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        if let R::A(reply) = std::mem::replace(&mut x.remote, R::new(ch, ws, ps)) {
            reply.send(Ok(self_));
            Ok(())
        } else {
            Err(ConnectionError::ChannelOpenUnexpected)
        }
    }

    pub fn push_open_failure(
        &mut self,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        if let R::A(reply) = std::mem::replace(&mut x.remote, R::new(0, 0, 0)) {
            reply.send(Err(reason));
            Ok(())
        } else {
            Err(ConnectionError::ChannelOpenUnexpected)
        }
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        let len = data.len() as u32;
        if x.local_window_size < len {
            return Err(ConnectionError::ChannelWindowSizeExceeded);
        }
        if x.local_max_packet_size < len {
            return Err(ConnectionError::ChannelMaxPacketSizeExceeded);
        }
        if x.data_in.write(data) < data.len() {
            return Err(ConnectionError::ChannelBufferSizeExceeded);
        }
        x.outer_task.take().map(Waker::wake).unwrap_or(());
        Ok(())
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        Ok(())
    }

    pub fn push_request(&mut self, request: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_success(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_failure(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }

    pub fn terminate(&mut self, e: ConnectionError) {}

    pub fn poll<T: TransportLayer>(
        &self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        Poll::Pending
    }
}

impl Read for DirectTcpIp {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut x = (self.0).0.lock().unwrap();
        let read = x.data_in.read(buf);
        if read > 0 {
            x.outer_task = None;
            Poll::Ready(Ok(read))
        } else if x.is_eof_received {
            x.outer_task = None;
            Poll::Ready(Ok(0))
        } else {
            x.register_outer_task(cx);
            Poll::Pending
        }
    }
}

impl ChannelState43 {
    fn register_outer_task(&mut self, cx: &mut Context) {
        if let Some(ref waker) = self.outer_task {
            if waker.will_wake(cx.waker()) {
                return;
            }
        }
        self.outer_task = Some(cx.waker().clone())
    }
}
