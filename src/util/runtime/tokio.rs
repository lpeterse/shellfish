pub use tokio::fs::File;
pub use tokio::net::TcpListener;
pub use tokio::net::TcpStream;
pub use tokio::net::UnixStream;
pub use tokio::task::spawn;

pub use tokio::io::AsyncRead;
pub use tokio::io::AsyncReadExt;
pub use tokio::io::AsyncWrite;
pub use tokio::io::AsyncWriteExt;
pub use tokio::time::sleep;

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Socket: std::fmt::Debug + AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static {
    #[inline]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>> {
        let mut buf = tokio::io::ReadBuf::new(buf);
        std::task::ready!(AsyncRead::poll_read(self, cx, &mut buf))?;
        Poll::Ready(Ok(buf.filled().len()))
    }
    #[inline]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        AsyncWrite::poll_write(self, cx, buf)
    }
    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        AsyncWrite::poll_flush(self, cx)
    }
    #[inline]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        AsyncWrite::poll_shutdown(self, cx)
    }
}

impl Socket for TcpStream {}
impl Socket for UnixStream {}
