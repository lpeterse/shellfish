use crate::transport::TransportLayer;

use std::sync::Arc;

pub trait Service {
    type Config: Send + Sync;
    type Transport: TransportLayer;

    const NAME: &'static str;

    fn new(config: &Arc<Self::Config>, transport: Self::Transport) -> Self;
}
