mod config;

pub use self::config::*;

use crate::agent::Agent;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::service::*;
use crate::transport::*;

use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;

pub struct Client {
    config: ClientConfig,
    user_name: Option<String>,
    agent: Option<Agent<Client>>,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(
        &self,
        addr: A,
    ) -> Result<Connection<Self>, ClientError> {
        let socket = TcpStream::connect(addr)
            .await
            .map_err(ClientError::ConnectError)?;
        let transport: Transport<Client, TcpStream> = Transport::new(&self.config, socket).await?;
        Ok(match self.user_name {
            None => Connection::new(
                transport
                    .request_service(<Connection<Self> as Service<Self>>::NAME)
                    .await?,
            ),
            Some(ref user) => {
                let transport = transport
                    .request_service(<UserAuth<Self> as Service<Self>>::NAME)
                    .await?;
                UserAuth::new(transport)
                    .authenticate(user, self.agent.clone())
                    .await?
            }
        })
    }

    pub fn config(&mut self) -> &mut ClientConfig {
        &mut self.config
    }

    pub fn user_name(&mut self) -> &mut Option<String> {
        &mut self.user_name
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            config: ClientConfig::default(),
            user_name: std::env::var("LOGNAME")
                .or_else(|_| std::env::var("USER"))
                .ok(),
            agent: Agent::new_env(),
        }
    }
}

#[derive(Debug)]
pub enum ClientError {
    ConnectError(std::io::Error),
    TransportError(TransportError),
    UserAuthError(UserAuthError),
    ConnectionError(ConnectionError),
}

impl From<TransportError> for ClientError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<UserAuthError> for ClientError {
    fn from(e: UserAuthError) -> Self {
        Self::UserAuthError(e)
    }
}

impl From<ConnectionError> for ClientError {
    fn from(e: ConnectionError) -> Self {
        Self::ConnectionError(e)
    }
}
