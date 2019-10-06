use crate::transport::HasTransport;
use crate::client::Client;

pub trait Role: HasTransport + Sized + Unpin + Send + Sync + 'static {
}

impl Role for Client {}
