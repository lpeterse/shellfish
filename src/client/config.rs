use crate::service::connection::ConnectionConfig;
use crate::transport::*;

use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub transport: Arc<TransportConfig>,
    pub connection: Arc<ConnectionConfig>,
}
