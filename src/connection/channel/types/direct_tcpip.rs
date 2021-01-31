mod open;

pub use self::open::*;

use super::super::*;
use crate::util::socket::Socket;
use std::io::Error;
use std::pin::Pin;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Debug)]
pub struct DirectTcpIp(pub(crate) ChannelHandle);

impl DirectTcpIp {
    pub fn interconnect<S: Socket>(self, socket: S) -> Interconnect<S> {
        self.0.interconnect(socket)
    }
}

impl Channel for DirectTcpIp {
    type Open = DirectTcpIpOpen;

    const NAME: &'static str = "direct-tcpip";

    fn new(channel: ChannelHandle) -> Self {
        DirectTcpIp(channel)
    }
}

impl AsyncRead for DirectTcpIp {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<Result<(), Error>> {
        Pin::new(&mut Pin::into_inner(self).0).poll_read(cx, buf)
    }
}

impl AsyncWrite for DirectTcpIp {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut Pin::into_inner(self).0).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut Pin::into_inner(self).0).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut Pin::into_inner(self).0).poll_shutdown(cx)
    }
}
