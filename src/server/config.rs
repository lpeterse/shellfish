use crate::agent::AuthAgent;
use crate::agent::LocalAgent;
use crate::connection::ConnectionConfig;
use crate::transport::*;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub socket: Arc<SocketConfig>,
    pub transport: Arc<TransportConfig>,
    pub connection: Arc<ConnectionConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket: Arc::new(SocketConfig::default()),
            transport: Arc::new(TransportConfig::default()),
            connection: Arc::new(ConnectionConfig::default()),
        }
    }
}


#[derive(Clone, Debug)]
pub struct SocketConfig {
    pub bind_addr: SocketAddr,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 2200),
        }
    }
}
