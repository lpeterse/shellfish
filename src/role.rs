use crate::transport::HasTransport;

pub trait Role: HasTransport + Sized + Unpin + Send + Sync + 'static {
}

pub struct Server {}
