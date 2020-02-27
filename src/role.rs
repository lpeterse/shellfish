use crate::client::{Client, ClientConfig};
use crate::server::{Server, ServerConfig};
use crate::transport::kex::{ClientKexMachine, KexMachine, ServerKexMachine};

pub trait Role: Sized + Unpin + Send + Sync + 'static {
    type Config;
    type KexMachine: KexMachine + Sized + Send + Unpin;
}

impl Role for Client {
    type Config = ClientConfig;
    type KexMachine = ClientKexMachine;
}

impl Role for Server {
    type Config = ServerConfig;
    type KexMachine = ServerKexMachine;
}
