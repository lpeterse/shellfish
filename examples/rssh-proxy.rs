use async_std::net::TcpListener;
use async_std::net::{SocketAddr, TcpStream};
use async_std::prelude::FutureExt;
use async_std::task::{block_on, sleep, spawn};
use rssh::util::oneshot;
use rssh::util::socks5;
use std::error::Error;
use std::sync::{Arc, Mutex};

use rssh::client::*;
use rssh::connection::*;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    block_on(async move {
        let hostname = std::env::args().skip(1).next().unwrap_or("localhost:22".into());
        let connection = SupervisedConnection::new(Client::default(), hostname);
        let listener: TcpListener = TcpListener::bind("127.0.0.1:1080").await?;
        loop {
            let (sock, addr) = listener.accept().await?;
            connection.proxy(sock, addr);
        }
    })
}

#[derive(Clone)]
pub struct SupervisedConnection {
    state: Arc<Mutex<Option<Connection>>>,
    canary: Arc<oneshot::Sender<()>>,
}

impl SupervisedConnection {
    const MIN_DELAY: u64 = 1;
    const MAX_DELAY: u64 = 300;

    pub fn new<H: Into<String>>(client: Client, hostname: H) -> Self {
        let (canary, canary_) = oneshot::channel();
        let canary = Arc::new(canary);
        let canary_ = async { canary_.await.unwrap_or(()) };
        let state = Arc::new(Mutex::new(None));
        let _ = spawn(Self::run(client, hostname.into(), state.clone()).race(canary_));
        Self { state, canary }
    }

    pub fn get(&self) -> Option<Connection> {
        self.state.lock().unwrap().clone()
    }

    async fn run(client: Client, hostname: String, state: Arc<Mutex<Option<Connection>>>) {
        let mut delay = 0;
        loop {
            let e = match client.connect(&hostname).await {
                Err(e) => e,
                Ok(c) => {
                    log::info!("Connection to {} established", hostname);
                    delay = 0;
                    *state.lock().unwrap() = Some(c.clone());
                    ConnectionError::from(c.await).into()
                }
            };
            *state.lock().unwrap() = None;
            delay = std::cmp::min(delay * 2, Self::MAX_DELAY);
            delay = std::cmp::max(delay, Self::MIN_DELAY);
            log::warn!("Connection to {} failed: {}", hostname, e);
            log::info!("Scheduled reconnect in {} seconds", delay);
            sleep(std::time::Duration::from_secs(delay)).await
        }
    }

    fn proxy(&self, sock: TcpStream, addr: SocketAddr) {
        let self_ = self.clone();
        let _ = spawn(async move {
            if let Err(e) = self_.proxy_(sock, addr).await {
                log::warn!("Proxy connection failed: {}", e)
            }
        });
    }

    async fn proxy_(
        &self,
        sock: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let cr = socks5::serve(sock).await?;
        let dh = cr.host().to_string();
        let dp = cr.port();
        let sa = addr.ip();
        let sp = addr.port();
        let conn = self.get().ok_or("no connection to proxy")?;
        let chan = conn.open_direct_tcpip(dh, dp, sa, sp).await??;
        let sock = cr.accept(addr).await?;
        chan.interconnect(sock).await?;
        Ok(())
    }
}
