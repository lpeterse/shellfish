use super::interconnect::Interconnect;
use super::state::*;
use super::*;
use crate::util::socket::Socket;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

macro_rules! wake {
    ($state:ident) => {
        let mut state: MutexGuard<ChannelState> = $state;
        let w = state.inner_task_waker.take();
        drop(state);
        if let Some(waker) = w {
            waker.wake()
        }
    };
}

macro_rules! pend {
    ($state:ident, $cx:ident, $ev:expr) => {
        $state.outer_task_flags |= $ev;
        $state.outer_task_waker = Some($cx.waker().clone())
    };
}

#[derive(Debug)]
pub struct ChannelHandle(Arc<Mutex<ChannelState>>);

impl ChannelHandle {
    pub(crate) fn new(state: Arc<Mutex<ChannelState>>) -> Self {
        Self(state)
    }

    pub fn interconnect<S: Socket>(self, socket: S) -> Interconnect<S> {
        Interconnect::new(self, socket)
    }
}

impl AsyncRead for ChannelHandle {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<std::io::Result<()>> {
        let mut channel = self.0.lock().unwrap();
        if channel.stdin.is_empty() {
            if channel.eof_rcvd {
                // The channel has been terminated gracefully.
                Poll::Ready(Ok(()))
            } else if channel.close_rcvd {
                // The channel has been terminated without EOF.
                Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "")))
            } else {
                // The channel is open: Wait for more data, EOF or close by remote.
                pend!(channel, cx, EV_READABLE | EV_EOF_RCVD | EV_CLOSE_RCVD);
                Poll::Pending
            }
        } else {
            // The number of bytes to read is limited by bytes available and supplied buffer size.
            let n = std::cmp::min(channel.stdin.len(), buf.remaining());
            let b = buf.initialize_unfilled_to(n);
            let m = channel.stdin.read(b);
            assert!(n == m);
            buf.advance(n);
            // The inner task only needs to be woken if window adjust is recommended.
            if channel.recommended_window_adjust().is_some() {
                wake!(channel);
            }
            Poll::Ready(Ok(()))
        }
    }
}

impl AsyncWrite for ChannelHandle {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let mut channel = self.0.lock().unwrap();
        if channel.eof{
            Poll::Ready(Err(Error::new(ErrorKind::Other, "write after shutdown")))
        } else if channel.close_rcvd {
            Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "closed by remote")))
        } else if channel.rws > 0 && channel.stdout.len() < channel.mbs {
            let n = buf.len();
            let n = std::cmp::min(n, channel.mbs - channel.stdout.len());
            let n = std::cmp::min(n, channel.rws as usize);
            channel.stdout.write_all(&buf[..n]);
            channel.rws -= n as u32;
            if n < buf.len() {
                // Wake inner task only if less bytes written than requested.
                // Inner task is required to try sending even without explicit flush in this case.
                // Inner task shall intentionally not be woken for every small chunk written.
                wake!(channel);
            }
            Poll::Ready(Ok(n))
        } else {
            pend!(channel, cx, EV_WRITABLE | EV_CLOSE_RCVD);
            Poll::Pending
        }
    }

    /// Flushing just waits until all data has been sent.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        let mut channel = self.0.lock().unwrap();
        if channel.stdout.is_empty() {
            // All pending data has been transmitted.
            Poll::Ready(Ok(()))
        } else if channel.close_rcvd {
            // The remote side closed the channel before we could send all data.
            Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "closed by remote")))
        } else {
            // Wake us when either all data has been transmitted or remote closed the channel.
            pend!(channel, cx, EV_FLUSHED | EV_CLOSE_RCVD);
            Poll::Pending
        }
    }

    /// Shutdown causes sending an `SSH_MSG_CHANNEL_EOF` (meaning that there won't be any more
    /// data).
    ///
    /// The internal connection handler will first transmit any pending data and then signal eof.
    /// `SSH_MSG_CHANNEL_CLOSE` is sent automatically on [drop].
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        let mut channel = self.0.lock().unwrap();
        channel.eof = true;
        if channel.eof_sent {
            // This implies complete transmission of any pending data.
            Poll::Ready(Ok(()))
        } else if channel.close_rcvd {
            // The remote side closed the channel before we could send EOF.
            Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "closed by remote")))
        } else {
            // Wake us when either EOF has been sent or remote closed the channel.
            pend!(channel, cx, EV_EOF_SENT | EV_CLOSE_RCVD);
            Poll::Pending
        }
    }
}

/// Dropping a [ChannelHandle] initiates the channel close procedure. Pending data will be
/// transmitted before sending an `SSH_MSG_CHANNEL_CLOSE`. The channel gets freed after
/// `SSH_MSG_CHANNEL_CLOSE` has been sent _and_ received. Of course, the [drop] call itself does
/// not block and return immediately.
impl Drop for ChannelHandle {
    fn drop(&mut self) {
        let mut channel = self.0.lock().unwrap();
        channel.close = true;
        wake!(channel);
    }
}
