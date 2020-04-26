use crate::service::connection::ConnectionConfig;
use crate::transport::*;

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

#[derive(Clone, Debug)]
pub struct TcpKeepaliveConfig {
    pub time: Option<std::time::Duration>,
    pub intvl: Option<std::time::Duration>,
    pub probes: Option<usize>,
}

impl Default for TcpKeepaliveConfig {
    fn default() -> Self {
        Self {
            time: Some(std::time::Duration::from_secs(300)),
            intvl: Some(std::time::Duration::from_secs(5)),
            probes: Some(5),
        }
    }
}
