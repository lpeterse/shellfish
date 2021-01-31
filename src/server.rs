mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::*;
use crate::util::role::Role;
use tokio::net::TcpListener;
use std::sync::Arc;

#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
    agent: Arc<dyn AuthAgent>,
    listener: TcpListener,
}

impl Role for Server {}
