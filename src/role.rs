use crate::client::Client;
use crate::server::Server;

pub trait Role: Sized + Unpin + Send + Sync + 'static {}

impl Role for Client {}

impl Role for Server {}
