mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::Agent;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::service::*;
use crate::transport::*;

use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;

pub struct Client {
    config: ClientConfig,
    username: Option<String>,
    agent: Option<Agent>,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(
        &self,
        addr: A,
    ) -> Result<Connection<Self>, ClientError> {
        let e = ClientError::ConnectError;
        let socket = TcpStream::connect(addr).await.map_err(e)?;
        let transport: Transport<Client, TcpStream> = Transport::new(&self.config, socket).await?;
        Ok(match self.username {
            None => {
                let service = <Connection<Self> as Service<Self>>::NAME;
                let transport = transport.request_service(service).await?;
                Connection::new(&self.config, transport)
            }
            Some(ref user) => {
                let agent = self.agent.clone();
                let service = <UserAuth<Self> as Service<Self>>::NAME;
                let transport = transport.request_service(service).await?;
                UserAuth::new(&self.config, transport)
                    .authenticate(&self.config, user, agent)
                    .await?
            }
        })
    }

    pub fn config(&mut self) -> &mut ClientConfig {
        &mut self.config
    }

    pub fn username(&mut self) -> &mut Option<String> {
        &mut self.username
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
        }
    }
}
