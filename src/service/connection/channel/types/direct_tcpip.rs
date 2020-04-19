mod open;

use super::super::*;

pub use self::open::*;

use async_std::io::{Read, Write};
use async_std::task::Context;
use std::io::Error;
use std::pin::Pin;

#[derive(Debug)]
pub struct DirectTcpIp(pub(crate) ChannelHandle);

impl Channel for DirectTcpIp {
    type Open = DirectTcpIpOpen;
    //    type Request = DirectTcpIpRequest;

    const NAME: &'static str = "direct-tcpip";

    fn new(channel: ChannelHandle) -> Self {
        DirectTcpIp(channel)
    }
}

impl Read for DirectTcpIp {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut Pin::into_inner(self).0).poll_read(cx, buf)
    }
}

impl Write for DirectTcpIp {
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

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut Pin::into_inner(self).0).poll_close(cx)
    }
}
