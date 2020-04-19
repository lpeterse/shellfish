use async_std::io::{Read, ReadExt, Write};
use async_std::net::TcpStream;
use async_std::os::unix::net::UnixStream;

pub trait Socket:
    std::fmt::Debug + Read + ReadExt + Write + Unpin + Send + 'static
{
}

impl Socket for TcpStream {}
impl Socket for UnixStream {}
