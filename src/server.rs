mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::AuthAgent;
use crate::connection::Connection;
use crate::connection::ConnectionHandler;
use crate::transport::Transport;
use crate::user_auth::{UserAuth, UserAuthSession};
use crate::util::BoxFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinSet;

#[derive(Debug)]
pub struct Server {
    // config: Arc<ServerConfig>,
    // thread: tokio::task::JoinHandle<()>,
}

pub trait ServerHandler: Send + Sync + 'static {
    type Identity: Send + 'static;

    fn on_error(&self, err: ServerError);
    fn on_accept(
        &self,
        addr: SocketAddr,
    ) -> BoxFuture<Option<Box<dyn UserAuthSession<Identity = Self::Identity>>>>;
    fn on_authenticated(&self, identity: Self::Identity) -> BoxFuture<Box<dyn ConnectionHandler>>;
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
        let mut startups = JoinSet::<Option<Connection>>::new();
        let mut established = JoinSet::<()>::new();

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
                x = startups.join_next(), if !startups.is_empty() => {
                    log::debug!("number of unauthenticated connections: {}", startups.len());
                    if let Some(Ok(Some(connection))) = x {
                        established.spawn(connection.closed_fixme());
                    }
                }
                x = established.join_next(), if!established.is_empty() => {
                    log::debug!("numer of establied connections: {}", established.len());
                    drop(x)
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
) -> Option<Connection> {
    let c = &config;
    let h = &handler;
    let auth_session = handler.on_accept(addr).await?;
    handle_(c, h, &auth_agent, auth_session, sock).await.ok()
}

async fn handle_<Identity: 'static + Send>(
    config: &Arc<ServerConfig>,
    handler: &Arc<dyn ServerHandler<Identity = Identity>>,
    auth_agent: &Arc<dyn AuthAgent>,
    auth_session: Box<dyn UserAuthSession<Identity = Identity>>,
    sock: TcpStream,
) -> Result<Connection, ServerError> {
    let sv1 = UserAuth::SSH_USERAUTH;
    let sv2 = UserAuth::SSH_CONNECTION;
    let aa = &auth_agent;
    let ct = &config.transport;
    let cc = &config.connection;
    let mut tp = Transport::accept(sock, ct, aa, sv1).await?;
    let id = UserAuth::authenticate(auth_session, &mut tp, sv2).await?;
    let ch = handler.on_authenticated(id).await;
    let cn = Connection::new(cc, tp, ch);
    Ok(cn)
}
