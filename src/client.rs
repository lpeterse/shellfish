mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::connection::*;
use crate::transport::*;
use crate::user_auth::*;
use std::sync::Arc;
use tokio::net::TcpStream;

/// The client is a connection factory.
///
/// The client creates a new connection for each call to [connect](Self::connect). It is not coupled
/// with the connection and may be used several times in order to establish connections to different
/// hosts.
#[derive(Clone, Debug)]
pub struct Client {
    config: Arc<ClientConfig>,
}

impl Client {
    /// Create a new client with given config.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Get a reference on the configuration used by this client.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Create a new connection to the given host.
    ///
    /// A running `ssh-agent` is expected for key or certificate authentication (`SSH_AUTH_SOCK`
    /// environment variable).
    pub async fn connect(
        &self,
        user: &str,
        host: &str,
        port: u16,
        handler: Box<dyn ConnectionHandler>,
    ) -> Result<Connection, ClientError> {
        let e = |e: std::io::Error| TransportError::from(e);
        let socket = TcpStream::connect((host, port)).await.map_err(e)?;
        let tc = &self.config.transport;
        let cc = &self.config.connection;
        let hv = &self.config.host_verifier;
        let aa = &self.config.auth_agent;
        let sv = UserAuth::SSH_USERAUTH;
        let t = Transport::connect(socket, tc, hv, host, port, sv).await?;
        Ok(UserAuth::request_connection(t, cc, handler, user, aa).await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            config: Arc::new(ClientConfig::default()),
        }
    }
}
