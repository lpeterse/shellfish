mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::*;
use crate::transport::DefaultTransport;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct Server {
    config: Arc<ServerConfig>,
}

pub trait ServerHandler: Send + Sync + 'static {}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub async fn listen(&self) -> Result<(), ServerError> {
        let fe = |e: std::io::Error| ServerError::SocketError(e);
        let ba = self.config.socket.bind_addr;
        let tl = TcpListener::bind(ba).await.map_err(fe)?;
        loop {
            let (s, addr) = tl.accept().await.map_err(fe)?;
            let ct = &self.config.transport;
            let ca = &self.config.auth_agent;
            let t = DefaultTransport::accept(ct, s, ca).await?;
            log::warn!("ACCEPTED");
        }
        Ok(())
    }
}
