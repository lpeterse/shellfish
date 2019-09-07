use crate::transport::*;
use crate::codec::*;

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

        let mut transport: Transport<TcpStream> = Transport::new(stream, Role::Client).await?;

        println!("CONNECTED: {:?}", "ASD");
        transport.send(&ServiceRequest("ssh-userauth")).await?;
        transport.flush().await?;
        println!("ABC");

        async_std::task::sleep(std::time::Duration::from_secs(300)).await;

        loop {
            async_std::task::sleep(std::time::Duration::from_secs(5)).await;
            transport.rekey().await?;
        }

        Ok(Connection {
            transport
        })
    }
}

pub enum ConnectError {
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

pub struct Connection {
    transport: Transport<TcpStream>,
}

impl Connection {

}

pub struct Session {

}

pub struct ServiceRequest<'a> (&'a str);

impl <'a> ServiceRequest<'a> {
    pub const MSG_NUMBER: u8 = 5;
}

impl <'a> Codec<'a> for ServiceRequest<'a> {
    fn size(&self) -> usize {
        1 + Codec::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER);
        Codec::encode(&self.0, c);
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Some(Self(Codec::decode(c)?))
    }
}

