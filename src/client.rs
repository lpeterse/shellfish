use crate::transport::*;
use crate::service::user_auth::*;
use crate::service::connection::*;

use std::net::{ToSocketAddrs};
use async_std::net::{TcpStream};

pub struct Config {

}

pub struct Client {

}

impl Client {
    pub fn new(config: Config) -> Self {
        Client {}
    }

    pub async fn connect<A: ToSocketAddrs>(self, addr: A) -> Result<Connection, ConnectError> {
        let stream = TcpStream::connect(addr).await?;
        let transport: Transport<TcpStream> = Transport::new(stream, Role::Client).await?;
        Ok(Connection::new(transport))
    }
}

#[derive(Debug)]
pub enum ConnectError {
    ConnectError(std::io::Error),
    KexError(TransportError),
    UserAuthError(UserAuthError),
}

impl From<std::io::Error> for ConnectError {
    fn from(e: std::io::Error) -> Self {
        Self::ConnectError(e)
    }
}

impl From<UserAuthError> for ConnectError {
    fn from(e: UserAuthError) -> Self {
        Self::UserAuthError(e)
    }
}

impl From<TransportError> for ConnectError {
    fn from(e: TransportError) -> Self {
        Self::KexError(e)
    }
}

