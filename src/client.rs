mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::auth::*;
use crate::connection::*;
use crate::known_hosts::*;
use crate::transport::*;

use async_std::net::TcpStream;
use std::sync::Arc;

#[derive(Debug)]
pub struct Client {
    config: ClientConfig,
    username: Option<String>,
    auth_agent: Arc<dyn Agent>,
    known_hosts: Arc<dyn KnownHostsLike>,
}

impl Client {
    pub async fn connect<H: Into<String>>(&self, hostname: H) -> Result<Connection, ClientError> {
        let f = |e: std::io::Error| ClientError::ConnectError(e.kind());
        let hostname = hostname.into();
        let socket = TcpStream::connect(&hostname).await.map_err(f)?;
        if let Some(ref keepalive) = self.config.tcp.keepalive {
            keepalive.apply(&socket).map_err(f)?;
        }
        self.handle(socket, hostname).await
    }

    pub async fn handle(
        &self,
        socket: TcpStream,
        hostname: String,
    ) -> Result<Connection, ClientError> {
        let kh = self.known_hosts.clone();
        let tc = &self.config.transport;
        let cc = &self.config.connection;
        let t = DefaultTransport::connect(tc, &kh, hostname, socket).await?;
        let t = Box::new(t);
        Ok(match self.username {
            Some(ref user) => UserAuth::request(t, cc, user, &self.auth_agent).await?,
            None => {
                let n = <Connection as Service>::NAME;
                let t = TransportExt::request_service(t, n).await?;
                Connection::new(cc, t)
            }
        })
    }

    pub fn config(&mut self) -> &mut ClientConfig {
        &mut self.config
    }

    pub fn username(&mut self) -> &mut Option<String> {
        &mut self.username
    }

    pub fn auth_agent(&mut self) -> &mut Arc<dyn Agent> {
        &mut self.auth_agent
    }

    pub fn known_hosts(&mut self) -> &mut Arc<dyn KnownHostsLike> {
        &mut self.known_hosts
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
            known_hosts: Arc::new(KnownHosts::default()),
        }
    }
}
