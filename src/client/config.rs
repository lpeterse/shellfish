use crate::agent::*;
use crate::connection::ConnectionConfig;
use crate::host::*;
use crate::transport::*;
use crate::util::socket::*;

use std::sync::Arc;

/// The client configuration containing user name, agent, transport, connection properties etc.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub(crate) socket: Arc<SocketConfig>,
    pub(crate) transport: Arc<TransportConfig>,
    pub(crate) connection: Arc<ConnectionConfig>,
    pub(crate) host_verifier: Arc<dyn HostVerifier>,
    pub(crate) auth_agent: Arc<dyn AuthAgent>,
}

impl ClientConfig {
    pub fn socket(&self) -> &SocketConfig {
        &self.socket
    }

    pub fn socket_mut(&mut self) -> &mut SocketConfig {
        Arc::make_mut(&mut self.socket)
    }

    pub fn transport(&self) -> &TransportConfig {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut TransportConfig {
        Arc::make_mut(&mut self.transport)
    }

    pub fn connection(&self) -> &ConnectionConfig {
        &self.connection
    }

    pub fn connection_mut(&mut self) -> &mut ConnectionConfig {
        Arc::make_mut(&mut self.connection)
    }

    pub fn host_verifier_mut(&mut self) -> &mut Arc<dyn HostVerifier> {
        &mut self.host_verifier
    }

    pub fn auth_agent_mut(&mut self) -> &mut Arc<dyn AuthAgent> {
        &mut self.auth_agent
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            socket: Arc::new(SocketConfig::default()),
            transport: Arc::new(TransportConfig::default()),
            connection: Arc::new(ConnectionConfig::default()),
            host_verifier: Arc::new(KnownHosts::default()),
            auth_agent: match LocalAgent::new_env() {
                Some(agent) => Arc::new(agent),
                None => Arc::new(()),
            },
        }
    }
}
