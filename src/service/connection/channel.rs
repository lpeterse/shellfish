mod direct_tcpip;
mod session;

pub use self::direct_tcpip::*;
pub use self::session::*;

use super::*;
use crate::buffer::*;

use async_std::io::Read;
use async_std::task::Poll;
use async_std::task::Waker;
use std::sync::{Arc, Mutex};

pub trait ChannelOpen: Sized {
    type Open: std::fmt::Debug + Clone + Encode + Decode;
    type Confirmation: Encode + Decode;
}

pub(crate) trait Channel: ChannelOpen {
    type Request: ChannelRequest + Encode;

    const NAME: &'static str;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

type OpenReply =
    oneshot::Sender<Result<Result<ChannelState, ChannelOpenFailureReason>, ConnectionError>>;

#[derive(Clone, Debug)]
pub(crate) struct ChannelState(Arc<Mutex<ChannelStateInner>>);

#[derive(Debug)]
pub(crate) struct ChannelStateInner {
    local_id: u32,
    local_window_size: u32,
    local_max_window_size: u32,
    local_max_packet_size: u32,

    remote_id: u32,
    remote_window_size: u32,
    remote_max_packet_size: u32,

    eof_rx: bool,
    eof_tx: Option<bool>,

    close_rx: bool,
    close_tx: Option<bool>,

    inner_task: Option<Waker>,
    inner_error: Option<ConnectionError>,
    outer_task: Option<Waker>,
    outer_done: Option<()>,

    data_in: Buffer,
    data_out: Buffer,
    data_err: Buffer,

    open_reply: Option<OpenReply>,
}

impl ChannelState {
    pub fn new(
        local_id: u32,
        local_max_window_size: u32,
        local_max_packet_size: u32,
        reply: oneshot::Sender<Result<Result<Self, ChannelOpenFailureReason>, ConnectionError>>,
    ) -> Self {
        log::debug!(
            "Channel {}: Created (mws: {}, mps: {})",
            local_id,
            local_max_window_size,
            local_max_packet_size
        );
        Self(Arc::new(Mutex::new(ChannelStateInner {
            local_id,
            local_max_window_size,
            local_max_packet_size,
            local_window_size: local_max_window_size,

            remote_id: 0,
            remote_window_size: 0,
            remote_max_packet_size: 0,

            eof_rx: false,
            eof_tx: None,

            close_rx: false,
            close_tx: None,

            inner_task: None,
            inner_error: None,
            outer_task: None,
            outer_done: None,

            data_in: Buffer::new(0),
            data_out: Buffer::new(0),
            data_err: Buffer::new(0),

            open_reply: Some(reply),
        })))
    }

    pub fn push_open_confirmation(
        &mut self,
        ch: u32,
        ws: u32,
        ps: u32,
    ) -> Result<(), ConnectionError> {
        let self_ = self.clone();
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        if let Some(reply) = x.open_reply.take() {
            x.remote_id = ch;
            x.remote_window_size = ws;
            x.remote_max_packet_size = ps;
            reply.send(Ok(Ok(self_)));
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
        if let Some(reply) = x.open_reply.take() {
            x.eof_rx = true;
            x.eof_tx = Some(true);
            reply.send(Ok(Err(reason)));
            Ok(())
        } else {
            Err(ConnectionError::ChannelOpenUnexpected)
        }
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.assume_open()?;
        let len = data.len() as u32;
        if x.local_window_size < len {
            return Err(ConnectionError::ChannelWindowSizeExceeded);
        }
        if x.local_max_packet_size < len {
            return Err(ConnectionError::ChannelMaxPacketSizeExceeded);
        }
        x.data_in.write_all(data);
        x.outer_task.take().map(Waker::wake).unwrap_or(());
        Ok(())
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.assume_open()?;
        if code != 1 {
            return Err(ConnectionError::ChannelExtendedDataCodeInvalid);
        }
        let len = data.len() as u32;
        if x.local_window_size < len {
            return Err(ConnectionError::ChannelWindowSizeExceeded);
        }
        if x.local_max_packet_size < len {
            return Err(ConnectionError::ChannelMaxPacketSizeExceeded);
        }
        x.data_in.write_all(data);
        x.outer_task.take().map(Waker::wake).unwrap_or(());
        Ok(())
    }

    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.assume_open()?;
        x.eof_rx = true;
        x.wake_outer_task();
        Ok(())
    }

    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.assume_open()?;
        x.close_rx = true;
        x.wake_outer_task();
        Ok(())
    }

    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.assume_open()?;
        if (n as u64 + x.remote_window_size as u64) < (u32::MAX as u64) {
            x.remote_window_size += n;
            x.wake_outer_task();
            Ok(())
        } else {
            Err(ConnectionError::ChannelWindowAdjustOverflow)
        }
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

    pub fn poll<T: TransportLayer>(
        &self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        use std::ops::DerefMut;
        let mut guard = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        let x: &mut ChannelStateInner = guard.deref_mut();
        if x.open_reply.is_none() {
            while !x.data_out.is_empty() {
                let len = std::cmp::min(x.remote_max_packet_size, x.data_out.len() as u32);
                let len = std::cmp::min(x.remote_window_size, len);
                let data = &x.data_out.as_ref()[..len as usize];
                let msg = MsgChannelData::new(x.remote_id, data);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!(
                    "Channel {}: Sent MSG_CHANNEL_DATA ({} bytes)",
                    x.local_id,
                    len
                );
                x.remote_window_size -= len;
                x.data_out.consume(len as usize);
                x.wake_outer_task();
            }
            if let Some(false) = x.eof_tx {
                let msg = MsgChannelEof::new(x.remote_id);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("Channel {}: Sent MSG_CHANNEL_EOF", x.local_id);
                x.eof_tx = Some(true);
            }
            if let Some(false) = x.close_tx {
                let msg = MsgChannelClose::new(x.remote_id);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("Channel {}: Sent MSG_CHANNEL_CLOSE", x.local_id);
                x.close_tx = Some(true);
            }
            if x.close_rx && x.close_tx == Some(true) {
                return Poll::Ready(Ok(()));
            }
        }
        x.register_inner_task(cx);
        Poll::Pending
    }
}

impl ChannelStateInner {
    fn assume_open(&self) -> Result<(), ConnectionError> {
        if self.open_reply.is_none() {
            Ok(())
        } else {
            Err(ConnectionError::ChannelIdInvalid)
        }
    }

    fn register_inner_task(&mut self, cx: &mut Context) {
        if let Some(ref waker) = self.inner_task {
            if waker.will_wake(cx.waker()) {
                return;
            }
        }
        self.inner_task = Some(cx.waker().clone())
    }

    fn register_outer_task(&mut self, cx: &mut Context) {
        if let Some(ref waker) = self.outer_task {
            if waker.will_wake(cx.waker()) {
                return;
            }
        }
        self.outer_task = Some(cx.waker().clone())
    }

    fn wake_inner_task(&mut self) {
        self.inner_task.take().map(Waker::wake).unwrap_or(())
    }

    fn wake_outer_task(&mut self) {
        self.outer_task.take().map(Waker::wake).unwrap_or(())
    }
}

impl Drop for ChannelStateInner {
    fn drop(&mut self) {
        log::debug!("Channel {}: Dropped", self.local_id);
    }
}

impl Terminate for ChannelState {
    fn terminate(&mut self, e: ConnectionError) {
        if let Ok(ref mut x) = self.0.lock() {
            x.terminate(e)
        }
    }
}

impl Terminate for ChannelStateInner {
    fn terminate(&mut self, e: ConnectionError) {
        if let Some(reply) = self.open_reply.take() {
            reply.send(Err(e));
        } else {
            self.inner_error = Some(e);
            self.outer_task.take().map(Waker::wake).unwrap_or(());
        }
    }
}
