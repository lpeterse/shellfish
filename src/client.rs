use crate::agent::Agent;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::service::*;
use crate::transport::*;

use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;

pub struct Client {
    user_name: Option<String>,
    agent: Option<Agent<Client>>,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(
        &self,
        addr: A,
    ) -> Result<Connection<Self>, ClientError> {
        let stream = TcpStream::connect(addr).await.map_err(ClientError::ConnectError)?;
        let config = TransportConfig::default();
        let transport: Transport<Client, TcpStream> = Transport::new(&config, stream).await?;
        let transport = match self.user_name {
            None => {
                transport
                    .request_service(<Connection<Self> as Service>::NAME)
                    .await?
            }
            Some(ref user) => {
                let transport = transport
                    .request_service(<UserAuth<Self> as Service>::NAME)
                    .await?;
                UserAuth::<Self>::authenticate(
                    transport,
                    <Connection<Self> as Service>::NAME,
                    user,
                    self.agent.clone(),
                )
                .await?
            }
        };

        Ok(Connection::<Self>::new(transport))
    }

    pub fn user_name(&mut self) -> &mut Option<String> {
        &mut self.user_name
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
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
