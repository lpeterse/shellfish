use crate::util::tcp::*;
use tokio::net::UnixStream;
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct SocketConfig {
    pub tcp_keepalive: Option<TcpKeepaliveConfig>,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            tcp_keepalive: Some(Default::default()),
        }
    }
}

pub trait Socket: std::fmt::Debug + AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static {}

impl Socket for TcpStream {}
impl Socket for UnixStream {}
