use futures::io::*;
use async_std::net::TcpStream;

pub trait Socket:
    AsyncRead + AsyncReadExt + AsyncWrite + Unpin + Send + 'static
{
}

impl Socket for TcpStream {}
