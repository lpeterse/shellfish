use crate::connection::ConnectionConfig;
use crate::transport::*;
use crate::util::tcp::*;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct ServerConfig {
    pub tcp: Arc<TcpConfig>,
    pub transport: Arc<TransportConfig>,
    pub connection: Arc<ConnectionConfig>,
}

#[derive(Clone, Debug)]
pub struct TcpConfig {
    pub bind_addr: SocketAddr,
    pub keepalive: Option<TcpKeepaliveConfig>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 2200),
            keepalive: Some(Default::default()),
        }
    }
}
