use crate::client::{Client, ClientConfig};
use crate::server::{Server, ServerConfig};
use crate::transport::kex::{ClientKex, Kex, ServerKex};

pub trait Role: Sized + Unpin + Send + Sync + 'static {
    type Config;
    type Kex: Kex + Sized + Send + Unpin;
}

impl Role for Client {
    type Config = ClientConfig;
    type Kex = ClientKex;
}

impl Role for Server {
    type Config = ServerConfig;
    type Kex = ServerKex;
}
