mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::AuthAgent;
use crate::connection::Connection;
use crate::transport::Transport;
use crate::user_auth::{UserAuth, UserAuthSession, authenticate};
use crate::util::BoxFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinSet;

#[derive(Debug)]
pub enum AuthResult {
    Accept,
    Reject
}

#[derive(Debug)]
pub struct Server {
    // config: Arc<ServerConfig>,
    // thread: tokio::task::JoinHandle<()>,
}

pub trait ServerHandler: Send + Sync + 'static {
    type Identity: Send + 'static;

    fn on_error(&self, err: ServerError);
    fn on_accept(&self, addr: SocketAddr) -> BoxFuture<Option<Box<dyn UserAuthSession<Identity = Self::Identity>>>>;
    fn on_connection(&self, connection: Connection, identity: Self::Identity);
}

impl Server {
    pub async fn run<Identity: Send + 'static>(
        config: Arc<ServerConfig>,
        handler: Arc<dyn ServerHandler<Identity = Identity>>,
        auth_agent: Arc<dyn AuthAgent>,
    ) -> Result<(), ServerError> {
        let fe = |e: std::io::Error| ServerError::SocketError(e);
        let ba = config.socket.bind_addr;
        let listener = TcpListener::bind(ba).await.map_err(fe)?;
        let mut startups = JoinSet::<()>::new();

        loop {
            tokio::select! {
                x = listener.accept() => {
                    let (sock, addr) = x.map_err(fe)?;
                    if startups.len() < 20 {
                        let c = config.clone();
                        let h = handler.clone();
                        let aa = auth_agent.clone();
                        let _ = startups.spawn(handle(c, h, aa, sock, addr));
                    } else {
                        drop(sock)
                    }
                }
                _ = startups.join_next(), if !startups.is_empty() => {
                    log::debug!("number of unauthenticated connections: {}", startups.len());
                    // just remove from set
                }
            };
        }
    }
}

async fn handle<Identity: 'static + Send>(
    config: Arc<ServerConfig>,
    handler: Arc<dyn ServerHandler<Identity = Identity>>,
    auth_agent: Arc<dyn AuthAgent>,
    sock: TcpStream,
    addr: SocketAddr,
) {
    let aa = &auth_agent;
    let ua = match handler.on_accept(addr).await {
        None => return,
        Some(ua) => ua
    };
    let ct = &config.transport;
    let cc = &config.connection;
    let sv = UserAuth::SSH_USERAUTH;
    let tp = match Transport::accept(sock, ct, aa, sv).await {
        Ok(x) => x,
        Err(e) => {
            log::warn!("{:?}", e);
            return;
        }
    };
    let identity = match authenticate(ua, tp).await {
        Ok(x) => x,
        Err(e) => {
            log::warn!("{:?}", e);
            return;
        }
    };
    log::error!("DONE");
}
