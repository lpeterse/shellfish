use crate::agent::Agent;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::service::*;
use crate::transport::*;

use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;

pub struct Client {
    user_name: Option<String>,
    agent: Option<Agent>,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<Connection, ConnectError> {
        let stream = TcpStream::connect(addr).await?;
        let transport: Transport<TcpStream> =
            Transport::new(Default::default(), stream, Role::Client).await?;
        let transport = match self.user_name {
            None => {
                transport
                    .request_service(<Connection as Service>::NAME)
                    .await?
            }
            Some(ref user) => {
                let transport = transport
                    .request_service(<UserAuth as Service>::NAME)
                    .await?;
                UserAuth::authenticate(
                    transport,
                    <Connection as Service>::NAME,
                    user,
                    self.agent.clone(),
                )
                .await?
            }
        };

        Ok(Connection::new(transport))
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
pub enum ConnectError {
    ConnectError(std::io::Error),
    TransportError(TransportError),
    UserAuthError(UserAuthError),
}

impl From<std::io::Error> for ConnectError {
    fn from(e: std::io::Error) -> Self {
        Self::ConnectError(e)
    }
}

impl From<TransportError> for ConnectError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<UserAuthError> for ConnectError {
    fn from(e: UserAuthError) -> Self {
        Self::UserAuthError(e)
    }
}
