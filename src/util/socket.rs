use std::future::poll_fn;
use std::pin::Pin;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpStream;
use tokio::net::UnixStream;

#[derive(Clone, Debug, Default)]
pub struct SocketConfig {}

pub trait Socket: std::fmt::Debug + AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static {}

impl Socket for TcpStream {}
impl Socket for UnixStream {}

pub async fn write_all<S: Socket>(socket: &mut S, buf: &[u8]) -> Result<(), std::io::Error> {
    let mut s = socket;
    let mut b = buf;
    while !b.is_empty() {
        let n = poll_fn(|cx| Pin::new(&mut s).poll_write(cx, b)).await?;
        b = &b[n..];
    }
    Ok(())
}

pub async fn read_exact<S: Socket>(socket: &mut S, buf: &mut [u8]) -> Result<(), std::io::Error> {
    let mut s = socket;
    let mut b = tokio::io::ReadBuf::new(buf);
    while b.filled().len() < b.capacity() {
        poll_fn(|cx| Pin::new(&mut s).poll_read(cx, &mut b)).await?;
    }
    Ok(())
}

pub async fn flush<S: Socket>(socket: &mut S) -> Result<(), std::io::Error> {
    let mut s = socket;
    poll_fn(|cx| Pin::new(&mut s).poll_flush(cx)).await
}
