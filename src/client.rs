mod config;
mod error;

pub use self::config::*;
pub use self::error::*;

use crate::agent::*;
use crate::host::*;
use crate::service::connection::*;
use crate::service::user_auth::*;
use crate::service::Service;
use crate::transport::*;

use async_std::net::TcpStream;
use std::sync::Arc;

#[derive(Debug)]
pub struct Client {
    config: ClientConfig,
    username: Option<String>,
    auth_agent: Arc<dyn AuthAgent>,
    hostkey_verifier: Arc<dyn HostKeyVerifier>,
}

impl Client {
    pub async fn connect<H: Into<String>>(&self, hostname: H) -> Result<Connection, ClientError> {
        let e = ClientError::ConnectError;
        let hostname = hostname.into();
        let socket = TcpStream::connect(&hostname).await.map_err(e)?;
        self.set_keepalive(&socket).map_err(e)?;
        self.handle(socket, hostname).await
    }

    pub async fn handle(
        &self,
        socket: TcpStream,
        hostname: String,
    ) -> Result<Connection, ClientError> {
        let verifier = self.hostkey_verifier.clone();
        let tc = &self.config.transport;
        let cc = &self.config.connection;
        let t = Transport::connect(tc, &verifier, hostname, socket).await?;
        Ok(match self.username {
            Some(ref user) => UserAuth::request(t, cc, user, &self.auth_agent).await?,
            None => {
                let n = <Connection as Service>::NAME;
                let t = TransportLayerExt::request_service(t, n).await?;
                Connection::new(cc, t)
            }
        })
    }

    pub fn config(&mut self) -> &mut ClientConfig {
        &mut self.config
    }

    pub fn username(&mut self) -> &mut Option<String> {
        &mut self.username
    }

    pub fn auth_agent(&mut self) -> &mut Arc<dyn AuthAgent> {
        &mut self.auth_agent
    }

    pub fn hostkey_verifier(&mut self) -> &mut Arc<dyn HostKeyVerifier> {
        &mut self.hostkey_verifier
    }

    // FIXME: Remove as soon as this is supported by std.
    fn set_keepalive(&self, socket: &TcpStream) -> Result<(), std::io::Error> {
        if let Some(ref keepalive) = self.config.tcp.keepalive {
            use libc::{c_int, c_void, socklen_t};
            use libc::{SOL_SOCKET, SOL_TCP, SO_KEEPALIVE};
            use libc::{TCP_KEEPCNT, TCP_KEEPIDLE, TCP_KEEPINTVL};
            use std::os::unix::io::AsRawFd;
            fn set_opt(sock: c_int, opt: c_int, val: c_int, payload: c_int) -> c_int {
                unsafe {
                    libc::setsockopt(
                        sock,
                        opt,
                        val,
                        &payload as *const c_int as *const c_void,
                        std::mem::size_of::<c_int>() as socklen_t,
                    )
                }
            }
            let fd = socket.as_raw_fd();
            let msg = "Setting TCP keepalive failed";
            let err = Err(std::io::Error::new(std::io::ErrorKind::Other, msg));

            if set_opt(fd, SOL_SOCKET, SO_KEEPALIVE, 1) != 0 {
                return err;
            }
            if let Some(x) = keepalive.time {
                if set_opt(fd, SOL_TCP, TCP_KEEPIDLE, x.as_secs() as c_int) != 0 {
                    return err;
                }
            }
            if let Some(x) = keepalive.intvl {
                if set_opt(fd, SOL_TCP, TCP_KEEPINTVL, x.as_secs() as c_int) != 0 {
                    return err;
                }
            }
            if let Some(x) = keepalive.probes {
                if set_opt(fd, SOL_TCP, TCP_KEEPCNT, x as c_int) != 0 {
                    return err;
                }
            }
        }
        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            config: ClientConfig::default(),
            username: std::env::var("LOGNAME")
                .or_else(|_| std::env::var("USER"))
                .ok(),
            auth_agent: match LocalAgent::new_env() {
                Some(agent) => Arc::new(agent),
                None => Arc::new(()),
            },
            hostkey_verifier: Arc::new(KnownHosts::default()),
        }
    }
}
