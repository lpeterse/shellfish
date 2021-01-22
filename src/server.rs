mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::*;
use crate::core::*;
use crate::util::runtime::TcpListener;
use std::sync::Arc;

#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
    agent: Arc<dyn AuthAgent>,
    listener: TcpListener,
}

impl Role for Server {}
