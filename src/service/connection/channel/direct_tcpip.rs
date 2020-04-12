mod open;

use super::*;

pub(crate) use self::open::*;

use async_std::io::{Read, Write};
use std::io::Result;

#[derive(Debug)]
pub struct DirectTcpIp(pub(crate) ChannelState);
pub enum DirectTcpIpRequest {}

impl ChannelOpen for DirectTcpIp {
    type Open = DirectTcpIpOpen;
    type Confirmation = ();
}

impl Channel for DirectTcpIp {
    type Request = DirectTcpIpRequest;

    const NAME: &'static str = "direct-tcpip";
}

impl ChannelRequest for DirectTcpIpRequest {
    fn name(&self) -> &'static str {
        unreachable!()
    }
}

impl Encode for DirectTcpIpRequest {
    fn size(&self) -> usize {
        unreachable!()
    }

    fn encode<E: Encoder>(&self, _e: &mut E) {
        unreachable!()
    }
}

impl Drop for DirectTcpIp {
    fn drop(&mut self) {
        let mut x = (self.0).0.lock().unwrap();
        x.close_tx = Some(false);
        x.wake_inner_task();
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
        } else if x.eof_rx {
            x.outer_task = None;
            Poll::Ready(Ok(0))
        } else {
            x.register_outer_task(cx);
            Poll::Pending
        }
    }
}

impl Write for DirectTcpIp {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        let mut x = (self.0).0.lock().unwrap();
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
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        let mut x = (self.0).0.lock().unwrap();
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
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        let mut x = (self.0).0.lock().unwrap();
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
