mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::auth::{Agent, LocalAgent, UserAuth, UserAuthRequest};
use crate::connection::*;
use crate::transport::*;
use crate::util::BoxFuture;

use async_std::net::{SocketAddr, TcpListener};
use std::sync::Arc;

#[derive(Debug)]
pub struct Server {
    config: ServerConfig,
    auth_agent: Arc<dyn Agent>,
    listener: TcpListener,
}

impl Server {
    pub async fn listen(config: ServerConfig) -> std::io::Result<Self> {
        let listener = TcpListener::bind(config.tcp.bind_addr).await?;
        let server = Self {
            config: config,
            listener,
            auth_agent: match LocalAgent::new_env() {
                Some(agent) => Arc::new(agent),
                None => Arc::new(()),
            },
        };
        Ok(server)
    }

    pub async fn accept(
        &self,
    ) -> std::io::Result<(
        BoxFuture<Result<UserAuthRequest<Connection>, ServerError>>,
        SocketAddr,
    )> {
        let (socket, addr) = self.listener.accept().await?;
        if let Some(ref keepalive) = self.config.tcp.keepalive {
            keepalive.apply(&socket)?;
        }
        let agent = self.auth_agent.clone();
        let tconfig = self.config.transport.clone();
        let cconfig = self.config.connection.clone();
        let future = Box::pin(async move {
            let t = Transport::accept(tconfig, agent, socket).await?;
            let r = UserAuth::offer(t, cconfig).await?;
            Ok(r)
        });
        Ok((future, addr))
    }
}
