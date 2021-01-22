use crate::util::tcp::*;

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
