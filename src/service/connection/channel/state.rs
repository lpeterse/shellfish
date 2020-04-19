use super::super::MsgChannelClose;
use super::super::MsgChannelData;
use super::super::MsgChannelEof;
use super::super::Terminate;
use super::*;

use crate::buffer::Buffer;
use crate::transport::TransportLayer;

use async_std::io::{Read, Write};
use async_std::task::Waker;
use async_std::task::{ready, Context};
use std::io::Error;
use std::pin::Pin;

#[derive(Debug)]
pub struct ChannelState {
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
}

impl ChannelState {
    pub fn new(lid: u32, lws: u32, lps: u32, rid: u32, rws: u32, rps: u32) -> Self {
        Self {
            local_id: lid,
            local_max_window_size: lws,
            local_max_packet_size: lps,
            local_window_size: lws,

            remote_id: rid,
            remote_window_size: rws,
            remote_max_packet_size: rps,

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
        }
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let len = data.len() as u32;
        if self.local_window_size < len {
            return Err(ConnectionError::ChannelWindowSizeExceeded);
        }
        if self.local_max_packet_size < len {
            return Err(ConnectionError::ChannelMaxPacketSizeExceeded);
        }
        self.data_in.write_all(data);
        self.outer_task.take().map(Waker::wake).unwrap_or(());
        Ok(())
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        if code != 1 {
            return Err(ConnectionError::ChannelExtendedDataCodeInvalid);
        }
        let len = data.len() as u32;
        if self.local_window_size < len {
            return Err(ConnectionError::ChannelWindowSizeExceeded);
        }
        if self.local_max_packet_size < len {
            return Err(ConnectionError::ChannelMaxPacketSizeExceeded);
        }
        self.data_in.write_all(data);
        self.outer_task.take().map(Waker::wake).unwrap_or(());
        Ok(())
    }

    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        self.eof_rx = true;
        self.wake_outer_task();
        Ok(())
    }

    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        self.close_rx = true;
        self.wake_outer_task();
        Ok(())
    }

    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        if (n as u64 + self.remote_window_size as u64) < (u32::MAX as u64) {
            self.remote_window_size += n;
            self.wake_outer_task();
            Ok(())
        } else {
            Err(ConnectionError::ChannelWindowAdjustOverflow)
        }
    }

    pub fn poll<T: TransportLayer>(
        &mut self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        if self.close_tx != Some(true) {
            while !self.data_out.is_empty() {
                let len = std::cmp::min(self.remote_max_packet_size, self.data_out.len() as u32);
                let len = std::cmp::min(self.remote_window_size, len);
                let data = &self.data_out.as_ref()[..len as usize];
                let msg = MsgChannelData::new(self.remote_id, data);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!(
                    "Channel {}: Sent MSG_CHANNEL_DATA ({} bytes)",
                    self.local_id,
                    len
                );
                self.remote_window_size -= len;
                self.data_out.consume(len as usize);
                self.wake_outer_task();
            }
            if let Some(false) = self.eof_tx {
                let msg = MsgChannelEof::new(self.remote_id);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("Channel {}: Sent MSG_CHANNEL_EOF", self.local_id);
                self.eof_tx = Some(true);
            }
            if let Some(false) = self.close_tx {
                let msg = MsgChannelClose::new(self.remote_id);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("Channel {}: Sent MSG_CHANNEL_CLOSE", self.local_id);
                self.close_tx = Some(true);
            }
        }
        if self.close_tx == Some(true) && self.close_rx {
            Poll::Ready(Ok(()))
        } else {
            self.register_inner_task(cx);
            Poll::Pending
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
        if let Some(ref task) = self.inner_task {
            task.wake_by_ref();
        }
    }

    fn wake_outer_task(&mut self) {
        if let Some(ref task) = self.outer_task {
            task.wake_by_ref();
        }
    }
}

impl Drop for ChannelState {
    fn drop(&mut self) {
        log::debug!("Channel {}: Dropped", self.local_id);
    }
}

impl Terminate for ChannelState {
    fn terminate(&mut self, e: ConnectionError) {
        self.inner_error = Some(e);
        self.wake_outer_task();
    }
}

impl Read for ChannelState {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let x = Pin::into_inner(self);
        let read = x.data_in.read(buf);
        if read > 0 {
            x.outer_task = None;
            Poll::Ready(Ok(read))
        } else if x.eof_rx {
            x.outer_task = None;
            Poll::Ready(Ok(0))
        } else {
            x.register_outer_task(cx);
            Poll::Pending
        }
    }
}

impl Write for ChannelState {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let x = Pin::into_inner(self);
        let l1 = x.data_out.len();
        let l2 = x.local_max_window_size as usize;
        assert!(l1 <= l2);
        let len = l2 - l1;
        if len == 0 {
            x.register_outer_task(cx);
            Poll::Pending
        } else {
            x.data_out.write_all(&buf[..len]);
            Poll::Ready(Ok(len))
        }
    }

    /// Flushing just waits until all data has been sent.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        let x = Pin::into_inner(self);
        if x.data_out.is_empty() && x.eof_tx != Some(false) {
            Poll::Ready(Ok(()))
        } else {
            x.register_outer_task(cx);
            Poll::Pending
        }
    }

    /// Closing the stream shall be translated to eof (meaning that there won't be any more data).
    /// The internal connection handler will first transmit any pending data and then signal eof.
    /// Close gets sent automatically on drop (after sending pending data and eventually eof).
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        let x = Pin::into_inner(self);
        match x.eof_tx {
            Some(true) => Poll::Ready(Ok(())),
            Some(false) => {
                x.register_outer_task(cx);
                Poll::Pending
            }
            None => {
                x.eof_tx = Some(false);
                x.wake_inner_task();
                Poll::Ready(Ok(()))
            }
        }
    }
}
