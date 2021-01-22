use super::state::*;
use super::*;
use crate::util::runtime::Socket;
use futures_util::io::{AsyncRead, AsyncWrite};
use std::io::Error;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Context;

#[derive(Debug)]
pub struct ChannelHandle(Arc<Mutex<ChannelState>>);

impl ChannelHandle {
    pub(crate) fn with_state<F, X>(&self, f: F) -> X
    where
        F: FnOnce(&mut ChannelState) -> X,
    {
        let (result, waker) = {
            let mut state = self.0.lock().unwrap();
            (f(&mut state), state.inner_task_waker())
        };
        if let Some(waker) = waker {
            waker.wake()
        }
        result
    }

    pub fn interconnect<S: Socket>(self, socket: S) -> Interconnect<S> {
        Interconnect::new(self, socket)
    }
}

impl From<&Arc<Mutex<ChannelState>>> for ChannelHandle {
    fn from(x: &Arc<Mutex<ChannelState>>) -> Self {
        Self(x.clone())
    }
}

impl AsyncRead for ChannelHandle {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.with_state(|x| {
            let read = x.std.rx.read(buf);
            if read > 0 {
                x.outer_task_waker = None;
                Poll::Ready(Ok(read))
            } else if x.reof {
                x.outer_task_waker = None;
                Poll::Ready(Ok(0))
            } else {
                x.register_outer_task(cx);
                Poll::Pending
            }
        })
    }
}

impl AsyncWrite for ChannelHandle {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        self.with_state(|x| {
            let l1 = x.max_buffer_size as usize - x.std.tx.len();
            let l2 = buf.len();
            let len = std::cmp::min(l1, l2);
            if len == 0 {
                x.register_outer_task(cx);
                Poll::Pending
            } else {
                x.std.tx.write_all(&buf[..len]);
                x.inner_task_wake = true;
                Poll::Ready(Ok(len))
            }
        })
    }

    /// Flushing just waits until all data has been sent.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        self.with_state(|x| {
            if x.std.tx.is_empty() && (!x.leof || x.leof_sent) {
                Poll::Ready(Ok(()))
            } else {
                x.inner_task_wake = true;
                x.register_outer_task(cx);
                Poll::Pending
            }
        })
    }

    /// Closing the stream shall be translated to eof (meaning that there won't be any more data).
    /// The internal connection handler will first transmit any pending data and then signal eof.
    /// Close gets sent automatically on drop (after sending pending data and eventually eof).
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        self.with_state(|x| {
            x.leof = true;
            if x.std.tx.is_empty() && (!x.leof || x.leof_sent) {
                Poll::Ready(Ok(()))
            } else {
                x.register_outer_task(cx);
                Poll::Pending
            }
        })
    }
}

impl Drop for ChannelHandle {
    fn drop(&mut self) {
        self.with_state(|x| {
            x.lclose = true;
            x.inner_task_wake = true;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// drop: Shall set lclose and wake the inner task
    #[test]
    fn test_channel_handle_drop_01() {
        let st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        let ch = st.handle();
        drop(ch);
        st.with_state(|x| {
            assert_eq!(x.lclose, true);
            assert_eq!(x.inner_task_wake, true);
        })
    }
}
