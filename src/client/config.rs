use crate::connection::ConnectionConfig;
use crate::transport::*;
use crate::util::tcp::*;

use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub tcp: Arc<TcpConfig>,
    pub transport: Arc<TransportConfig>,
    pub connection: Arc<ConnectionConfig>,
}

#[derive(Clone, Debug)]
pub struct TcpConfig {
    pub keepalive: Option<TcpKeepaliveConfig>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            keepalive: Some(Default::default()),
        }
    }
}
