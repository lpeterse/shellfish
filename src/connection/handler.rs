use super::Connection;
use super::ConnectionError;
use crate::util::BoxFuture;

pub trait ConnectionHandler: Send + Sync + 'static {
    fn on_disconnect(&mut self, c: &Connection, e: ConnectionError) -> BoxFuture<()> {
        Box::pin(std::future::ready(()))
    }
}

impl ConnectionHandler for () {}
