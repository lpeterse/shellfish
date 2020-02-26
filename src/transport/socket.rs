use async_std::io::{Read, ReadExt, Write};
use async_std::net::TcpStream;

pub trait Socket:
    Read + ReadExt + Write + Unpin + Send + 'static
{
}

impl Socket for TcpStream {}
