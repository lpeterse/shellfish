pub mod connection;
pub mod user_auth;

use crate::transport::TransportLayer;

use std::sync::Arc;

pub trait Service {
    type Config;

    const NAME: &'static str;

    fn new<T: TransportLayer>(config: &Arc<Self::Config>, transport: T) -> Self;
}
