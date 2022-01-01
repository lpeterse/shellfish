use super::super::super::super::connection::msg::*;
use super::super::super::channel::{Channel, ChannelState, OpenFailure, PollResult};
use super::super::super::error::ConnectionError;
use super::DirectTcpIp;
use crate::transport::Transport;
use crate::util::buffer::Buffer;
use crate::util::check;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const EV_ERROR: u8 = 1;
const EV_FLUSHED: u8 = 2;
const EV_READABLE: u8 = 4;
const EV_WRITABLE: u8 = 8;
const EV_EOF_SENT: u8 = 16;
const EV_EOF_RCVD: u8 = 32;
const EV_CLOSE_SENT: u8 = 64;
const EV_CLOSE_RCVD: u8 = 128;

macro_rules! pend {
    ($state:ident, $cx:ident, $ev:expr) => {
        $state.outer_task_flags |= $ev | EV_ERROR;
        $state.outer_task_waker = Some($cx.waker().clone())
    };
}

macro_rules! wake_inner {
    ($state:ident) => {{
        log::trace!("waker_inner 1");
        let mut state: MutexGuard<StateInner> = $state;
        let w = state.inner_task_waker.take();
        drop(state);
        let _ = w.map(Waker::wake);
    }};
}

macro_rules! wake_outer {
    ($state:ident, $ev:expr) => {{
        let mut state: MutexGuard<StateInner> = $state;
        if state.outer_task_flags & $ev != 0 {
            let w = state.outer_task_waker.take();
            drop(state);
            let _ = w.map(Waker::wake);
        } else {
            drop(state);
        }
    }};
}

#[derive(Debug, Clone)]
pub(crate) struct State(pub Arc<Mutex<StateInner>>);

impl State {
    pub(crate) fn new(lid: u32, lbs: u32, lps: u32, rid: u32, rws: u32, rps: u32) -> Self {
        Self(Arc::new(Mutex::new(StateInner {
            lbs: lbs as usize,

            lid,
            lws: lbs,
            lps,

            rid,
            rws,
            rps,

            eof_send: false,
            eof_sent: false,
            eof_rcvd: false,

            close_send: false,
            close_sent: false,
            close_rcvd: false,

            error: false,

            stdin: Buffer::new(0),
            stdout: Buffer::new(0),

            inner_task_waker: None,
            outer_task_waker: None,
            outer_task_flags: 0,
        })))
    }

    pub(crate) fn set_open_confirmation(&self, rid: u32, rws: u32, rps: u32) {
        let mut x = self.0.lock().unwrap();
        x.rid = rid;
        x.rws = rws;
        x.rps = rps;
    }

    pub(crate) fn msg_open(&self, data: Vec<u8>) -> MsgChannelOpen<&'static str> {
        let x = self.0.lock().unwrap();
        MsgChannelOpen {
            name: DirectTcpIp::NAME,
            sender_channel: x.lid,
            initial_window_size: x.lws,
            maximum_packet_size: x.lps,
            data,
        }
    }

    pub(crate) fn msg_open_confirmation(&self) -> MsgChannelOpenConfirmation {
        let x = self.0.lock().unwrap();
        MsgChannelOpenConfirmation {
            recipient_channel: x.rid,
            sender_channel: x.lid,
            initial_window_size: x.lws,
            maximum_packet_size: x.lps,
            specific: b"",
        }
    }

    pub(crate) fn msg_open_failure(&self, e: OpenFailure) -> MsgChannelOpenFailure {
        let x = self.0.lock().unwrap();
        MsgChannelOpenFailure::new(x.rid, e)
    }
}

// =================================================================================================
// METHODS FOR OUTER TASK
// =================================================================================================

impl State {
    /// Set the close flag and wake the inner task
    pub(crate) fn close(&self) {
        let mut x = self.0.lock().unwrap();
        x.close_send = true;
        wake_inner!(x)
    }
}

impl AsyncRead for State {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        let mut state = self.0.lock().unwrap();
        if state.error {
            Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionAborted, "")))
        } else if state.stdin.is_empty() {
            if state.eof_rcvd {
                // The channel has been terminated gracefully.
                Poll::Ready(Ok(()))
            } else if state.close_rcvd {
                // The channel has been terminated without EOF.
                Poll::Ready(Err(io::Error::new(io::ErrorKind::UnexpectedEof, "")))
            } else {
                // The channel is open: Wait for more data, EOF or close by remote.
                pend!(state, cx, EV_READABLE | EV_EOF_RCVD | EV_CLOSE_RCVD);
                Poll::Pending
            }
        } else {
            // The number of bytes to read is limited by bytes available and supplied buffer size.
            let n = std::cmp::min(state.stdin.len(), buf.remaining());
            let b = buf.initialize_unfilled_to(n);
            let m = state.stdin.read(b);
            assert!(n == m);
            buf.advance(n);
            // The inner task only needs to be woken if window adjust is recommended.
            if state.recommended_window_adjust().is_some() {
                wake_inner!(state);
            }
            Poll::Ready(Ok(()))
        }
    }
}

impl AsyncWrite for State {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let mut state = self.0.lock().unwrap();
        if state.error {
            Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionAborted, "")))
        } else if state.eof_send || state.close_rcvd {
            Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "")))
        } else if state.rws > 0 && state.stdout.len() < state.lbs {
            let n = buf.len();
            let n = std::cmp::min(n, state.lbs - state.stdout.len());
            let n = std::cmp::min(n, state.rws as usize);
            state.stdout.write_all(&buf[..n]);
            state.rws -= n as u32;
            wake_inner!(state);
            Poll::Ready(Ok(n))
        } else {
            pend!(state, cx, EV_WRITABLE | EV_CLOSE_RCVD);
            Poll::Pending
        }
    }

    /// Flushing just waits until all data has been sent.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        let mut state = self.0.lock().unwrap();
        if state.error {
            Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionAborted, "")))
        } else if state.stdout.is_empty() {
            // All pending data has been transmitted.
            Poll::Ready(Ok(()))
        } else if state.close_rcvd {
            // The remote side closed the channel before we could send all data.
            Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "")))
        } else {
            // Wake us when either all data has been transmitted or remote closed the channel.
            pend!(state, cx, EV_FLUSHED | EV_CLOSE_RCVD);
            wake_inner!(state);
            Poll::Pending
        }
    }

    /// Shutdown causes sending an `SSH_MSG_CHANNEL_EOF` (meaning that there won't be any more
    /// data).
    ///
    /// The internal connection handler will first transmit any pending data and then signal eof.
    /// `SSH_MSG_CHANNEL_CLOSE` is sent automatically on [drop].
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        let mut state = self.0.lock().unwrap();
        state.eof_send = true;
        if state.error {
            Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionAborted, "")))
        } else if state.eof_sent {
            // This implies complete transmission of any pending data.
            Poll::Ready(Ok(()))
        } else if state.close_rcvd {
            // The remote side closed the channel before we could send EOF.
            Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "")))
        } else {
            // Wake us when either EOF has been sent or remote closed the channel.
            pend!(state, cx, EV_EOF_SENT | EV_CLOSE_RCVD);
            wake_inner!(state);
            Poll::Pending
        }
    }
}

// =================================================================================================
// METHODS FOR INNER TASK
// =================================================================================================

impl ChannelState for State {
    fn on_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let mut state = self.0.lock().unwrap();
        let len = data.len() as u32;
        check(!state.eof_rcvd).ok_or(ConnectionError::ChannelDataUnexpected)?;
        check(!state.close_rcvd).ok_or(ConnectionError::ChannelDataUnexpected)?;
        check(len <= state.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
        check(len <= state.lps).ok_or(ConnectionError::ChannelPacketSizeExceeded)?;
        state.lws -= len;
        state.stdin.write_all(data);
        wake_outer!(state, EV_READABLE);
        Ok(())
    }

    fn on_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        let mut state = self.0.lock().unwrap();
        check(!state.close_rcvd).ok_or(ConnectionError::ChannelWindowAdjustUnexpected)?;
        if (n as u64 + state.rws as u64) > (u32::MAX as u64) {
            return Err(ConnectionError::ChannelWindowAdjustOverflow);
        }
        state.rws += n;
        wake_outer!(state, EV_WRITABLE);
        Ok(())
    }

    fn on_eof(&mut self) -> Result<(), ConnectionError> {
        let mut state = self.0.lock().unwrap();
        check(!state.eof_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        check(!state.close_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        state.eof_rcvd = true;
        wake_outer!(state, EV_EOF_RCVD);
        Ok(())
    }

    fn on_close(&mut self) -> Result<(), ConnectionError> {
        let mut state = self.0.lock().unwrap();
        check(!state.close_rcvd).ok_or(ConnectionError::ChannelCloseUnexpected)?;
        state.close_send = true;
        state.close_rcvd = true;
        wake_outer!(state, EV_CLOSE_RCVD);
        Ok(())
    }

    fn on_error(self: Box<Self>, _: &Arc<ConnectionError>) {
        let mut state = self.0.lock().unwrap();
        state.error = true;
        wake_outer!(state, EV_ERROR);
    }

    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<PollResult, ConnectionError>> {
        let mut state = self.0.lock().unwrap();
        let mut evs = 0;
        let mut pending = false;
        // Perform all the checks only in case the `inner_task_waker` is none which
        // implies it has either not been initialized or taken away by the outer thread
        // which means there is work to do
        if state.inner_task_waker.is_none() {
            // Send data as long as data is available and the remote window size allows
            check(state.rps > 0).ok_or(ConnectionError::ChannelPacketSizeInvalid)?;
            while !pending && !state.stdout.is_empty() {
                let len = state.stdout.len();
                let len = std::cmp::min(len, state.rps as usize);
                let dat = &state.stdout.as_ref()[..len];
                let msg = MsgChannelData::new(state.rid, dat);
                match t.poll_send(cx, &msg)? {
                    Poll::Pending => pending = true,
                    Poll::Ready(()) => {
                        log::trace!(">> {:?}", msg);
                        state.stdout.consume(len);
                        evs |= EV_WRITABLE;
                    }
                }
            }
            if state.stdout.is_empty() {
                evs |= EV_FLUSHED;
            }
            // Send eof if flag set and eof not yet sent
            if !pending && state.eof_send && !state.eof_sent {
                let msg = MsgChannelEof::new(state.rid);
                pending |= t.poll_send(cx, &msg)?.is_pending();
                state.eof_sent = true;
                evs |= EV_EOF_SENT;
            }
            // Send close if flag set and close not yet sent
            if !pending && state.close_send && !state.close_sent {
                let msg = MsgChannelClose::new(state.rid);
                pending |= t.poll_send(cx, &msg)?.is_pending();
                state.close_sent = true;
                evs |= EV_CLOSE_SENT;
            }
            // Send window adjust message when threshold is reached
            if !pending {
                if let Some(n) = state.recommended_window_adjust() {
                    let msg = MsgChannelWindowAdjust::new(state.rid, n);
                    pending |= t.poll_send(cx, &msg)?.is_pending();
                    state.lws += n;
                }
            }
            // Register the inner task for wakeup
            state.inner_task_waker = Some(cx.waker().clone());
        }
        // The channel is considered completely closed only after a close
        // message has been received and one has been sent
        let closed = state.close_rcvd && state.close_sent;
        wake_outer!(state, evs);
        if pending {
            Poll::Pending
        } else if closed {
            Poll::Ready(Ok(PollResult::Closed))
        } else {
            Poll::Ready(Ok(PollResult::Noop))
        }
    }
}

#[derive(Debug)]
pub(crate) struct StateInner {
    lbs: usize,
    lid: u32,
    lws: u32,
    lps: u32,

    rid: u32,
    rws: u32,
    rps: u32,

    eof_send: bool,
    eof_sent: bool,
    eof_rcvd: bool,

    close_send: bool,
    close_sent: bool,
    close_rcvd: bool,

    error: bool,

    stdin: Buffer,
    stdout: Buffer,

    inner_task_waker: Option<Waker>,
    outer_task_waker: Option<Waker>,
    outer_task_flags: u8,
}

impl StateInner {
    pub fn recommended_window_adjust(&mut self) -> Option<u32> {
        let threshold = self.lbs / 2;
        if (self.lws as usize) < threshold {
            let buffered = self.stdin.len();
            if buffered < threshold {
                let adjustment = self.lbs - std::cmp::max(self.lws as usize, buffered);
                return Some(adjustment as u32);
            }
        }
        None
    }
}
