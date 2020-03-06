mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::Agent;
use crate::host::*;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::transport::*;

use async_std::net::TcpStream;
use std::sync::Arc;

pub struct Client {
    config: ClientConfig,
    username: Option<String>,
    agent: Option<Agent>,
    hostkey_verifier: Arc<Box<dyn HostKeyVerifier>>,
}

impl Client {
    pub async fn connect<H: HostName>(&self, host: H) -> Result<Connection<Self>, ClientError> {
        let e = ClientError::ConnectError;
        let hostname = host.name();
        let socket = TcpStream::connect(host).await.map_err(e)?;
        self.handle(hostname, socket).await
    }

    pub async fn handle<S: Socket>(
        &self,
        hostname: String,
        socket: S,
    ) -> Result<Connection<Self>, ClientError> {
        let verifier = self.hostkey_verifier.clone();
        let t = Transport::<Client, S>::new(&self.config, verifier, hostname, socket).await?;
        Ok(match self.username {
            Some(ref user) => UserAuth::request(t, &self.config, user, self.agent.clone()).await?,
            None => Connection::request(t, &self.config).await?,
        })
    }

    pub fn config(&mut self) -> &mut ClientConfig {
        &mut self.config
    }

    pub fn username(&mut self) -> &mut Option<String> {
        &mut self.username
    }

    pub fn agent(&mut self) -> &mut Option<Agent> {
        &mut self.agent
    }

    pub fn hostkey_verifier(&mut self) -> &mut Arc<Box<dyn HostKeyVerifier>> {
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
            agent: Agent::new_env(),
            hostkey_verifier: Arc::new(Box::new(KnownHosts::default())),
        }
    }
}
