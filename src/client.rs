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

    pub async fn connect<A: ToSocketAddrs>(self, addr: A) -> Connection {
        let stream = TcpStream::connect(addr).await.unwrap();
        let transport: Transport = Transport::new(stream).await;
        Connection {
            transport
        }
    }
}

pub struct Connection {
    transport: Transport,
}

pub struct Session {

}