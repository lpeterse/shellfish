pub use async_std::fs::File;
pub use async_std::net::TcpListener;
pub use async_std::net::TcpStream;
pub use async_std::os::unix::net::UnixStream;
pub use async_std::task::sleep;
pub use async_std::task::spawn;
pub use futures_util::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Socket: std::fmt::Debug + AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static {
    #[inline]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>> {
        AsyncRead::poll_read(self, cx, buf)
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
        AsyncWrite::poll_close(self, cx)
    }
}

impl Socket for TcpStream {}
impl Socket for UnixStream {}
