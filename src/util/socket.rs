use async_std::io::{Read, ReadExt, Write};
use async_std::net::TcpStream;
use async_std::os::unix::net::UnixStream;

#[cfg(test)]
pub use self::dummy::DummySocket;

pub trait Socket: std::fmt::Debug + Read + ReadExt + Write + Unpin + Send + 'static {}

impl Socket for TcpStream {}
impl Socket for UnixStream {}

#[cfg(test)]
mod dummy {
    use super::*;

    use async_std::future::Future;
    use async_std::io::*;
    use async_std::stream::Stream;
    use async_std::sync::{channel, Receiver, Sender};
    use async_std::task::ready;
    use std::io::Result;
    use std::pin::Pin;
    use std::task::Context;
    use std::task::Poll;

    #[derive(Debug)]
    pub struct DummySocket {
        rx: Receiver<Vec<u8>>,
        rx_leftover: Vec<u8>,
        tx: Sender<Vec<u8>>,
        tx_buffer: Vec<u8>,
    }

    impl DummySocket {
        pub fn new() -> (Self, Self) {
            let (tx_1, rx_1) = channel(1);
            let (tx_2, rx_2) = channel(1);
            let s_1 = Self {
                rx: rx_1,
                rx_leftover: Vec::new(),
                tx: tx_2,
                tx_buffer: Vec::new(),
            };
            let s_2 = Self {
                rx: rx_2,
                rx_leftover: Vec::new(),
                tx: tx_1,
                tx_buffer: Vec::new(),
            };
            (s_1, s_2)
        }
    }

    impl Socket for DummySocket {}

    impl Read for DummySocket {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<Result<usize>> {
            if self.rx_leftover.is_empty() {
                if let Some(x) = ready!(Pin::new(&mut self.rx).poll_next(cx)) {
                    self.rx_leftover = x;
                } else {
                    return Poll::Ready(Ok(0));
                }
            }
            if self.rx_leftover.len() > buf.len() {
                let len = buf.len();
                let (a, b) = self.rx_leftover.split_at(len);
                buf.copy_from_slice(a);
                self.rx_leftover = Vec::from(b);
                Poll::Ready(Ok(len))
            } else {
                let len = self.rx_leftover.len();
                buf[..len].copy_from_slice(self.rx_leftover.as_ref());
                self.rx_leftover = Vec::new();
                Poll::Ready(Ok(len))
            }
        }
    }

    impl Write for DummySocket {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
            Box::pin(async {
                self.tx.send(Vec::from(buf)).await;
                Ok(buf.len())
            })
            .as_mut()
            .poll(cx)
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<()>> {
            let (tx, rx) = channel(1);
            self.rx = rx;
            self.tx = tx;
            Poll::Ready(Ok(()))
        }
    }
}
