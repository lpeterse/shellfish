pub mod connection;
pub mod user_auth;

use crate::transport::TransportLayer;

use std::sync::Arc;

pub trait Service {
    type Config;
    type Transport: TransportLayer;

    const NAME: &'static str;

    fn new(config: &Arc<Self::Config>, transport: Self::Transport) -> Self;
}
