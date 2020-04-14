mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::*;
use crate::host::*;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::transport::*;

use async_std::net::TcpStream;
use std::sync::Arc;

#[derive(Debug)]
pub struct Client {
    config: ClientConfig,
    username: Option<String>,
    auth_agent: Arc<dyn AuthAgent>,
    hostkey_verifier: Arc<dyn HostKeyVerifier>,
}

impl Client {
    pub async fn connect<H: HostName>(&self, host: H) -> Result<Connection, ClientError> {
        let e = ClientError::ConnectError;
        let hostname = host.name();
        let socket = TcpStream::connect(host).await.map_err(e)?;
        self.handle(hostname, socket).await
    }

    pub async fn handle<S: Socket>(
        &self,
        hostname: String,
        socket: S,
    ) -> Result<Connection, ClientError> {
        let verifier = self.hostkey_verifier.clone();
        let tc = &self.config.transport;
        let cc = &self.config.connection;
        let t = Transport::<S>::connect(tc, &verifier, hostname, socket).await?;
        Ok(match self.username {
            Some(ref user) => UserAuth::request(t, cc, user, &self.auth_agent).await?,
            None => Connection::request(cc, t).await?,
        })
    }

    pub fn config(&mut self) -> &mut ClientConfig {
        &mut self.config
    }

    pub fn username(&mut self) -> &mut Option<String> {
        &mut self.username
    }

    pub fn auth_agent(&mut self) -> &mut Arc<dyn AuthAgent> {
        &mut self.auth_agent
    }

    pub fn hostkey_verifier(&mut self) -> &mut Arc<dyn HostKeyVerifier> {
        &mut self.hostkey_verifier
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            config: ClientConfig::default(),
            username: std::env::var("LOGNAME")
                .or_else(|_| std::env::var("USER"))
                .ok(),
            auth_agent: match LocalAgent::new_env() {
                Some(agent) => Arc::new(agent),
                None => Arc::new(()),
            },
            hostkey_verifier: Arc::new(KnownHosts::default()),
        }
    }
}
