use crate::transport::*;
use crate::service::user_auth::*;
use crate::service::connection::*;
use crate::agent::Agent;

use std::net::{ToSocketAddrs};
use async_std::net::{TcpStream};

pub struct Config {
}

pub struct Client {
    agent: Agent
}

impl Client {
    pub fn new(config: Config, agent: Agent) -> Self {
        Client { agent }
    }

    pub async fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> Result<Connection, ConnectError> {
        let stream = TcpStream::connect(addr).await?;
        let transport: Transport<TcpStream> = Transport::new(stream, Role::Client).await?;
        Ok(Connection::new_authenticated(&mut self.agent, transport).await?)
    }
}

#[derive(Debug)]
pub enum ConnectError {
    NoAgent,
    ConnectError(std::io::Error),
    TransportError(TransportError),
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
