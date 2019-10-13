use crate::transport::HasTransport;
use crate::client::{Client, ClientConfig};

pub trait Role: HasTransport + Sized + Unpin + Send + Sync + 'static {
    type Config;
}

impl Role for Client {
    type Config = ClientConfig;
}
