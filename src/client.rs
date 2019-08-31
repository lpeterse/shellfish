use crate::transport::*;

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

        let stream = TcpStream::connect(addr)
            .await
            .map_err(ConnectError::ConnectError)?;

        let mut transport: Transport<TcpStream> = Transport::new(stream)
            .await
            .map_err(ConnectError::TransportError)?;

        let msg: Message<&'static [u8]> = Message(b"sdjkah");
        let mut commands = futures::future::ready(msg);

        Ok(Connection {
            transport
        })
    }
}

pub enum ConnectError {
    ConnectError(std::io::Error),
    TransportError(TransportError),
}

pub struct Connection {
    transport: Transport<TcpStream>,
}

impl Connection {

}

pub struct Session {

}