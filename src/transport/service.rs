use crate::transport::Transport;

use std::sync::Arc;

pub trait Service {
    type Config: Send + Sync;

    const NAME: &'static str;

    fn new(config: &Arc<Self::Config>, transport: Box<dyn Transport>) -> Self;
}
