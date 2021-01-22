mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::connection::*;
use crate::core::*;
use crate::transport::*;
use crate::user_auth::*;
use crate::util::runtime::TcpStream;
use std::sync::Arc;

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
    /// The user name and authentication methods are contained in the configuration and set to
    /// sensible defaults. By default, the user name will be extracted from the environment and
    /// a running `ssh-agent` is expected for key or certificate authentication (`SSH_AUTH_SOCK`
    /// environment variable).
    pub async fn connect<H: ConnectionHandler>(
        &self,
        user: &str,
        host: &str,
        port: u16,
        handler: H,
    ) -> Result<Connection, ClientError> {
        let e = |e: std::io::Error| TransportError::from(e);
        let socket = TcpStream::connect((host, port)).await.map_err(e)?;
        if let Some(ref keepalive) = self.config.socket.tcp_keepalive {
            keepalive.apply(&socket).map_err(e)?;
        }
        let tc = &self.config.transport;
        let cc = &self.config.connection;
        let hv = &self.config.host_verifier;
        let aa = &self.config.auth_agent;
        let t = DefaultTransport::connect(tc, socket, host, port, hv).await?;
        let t = GenericTransport::from(t);
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

impl Role for Client {}
