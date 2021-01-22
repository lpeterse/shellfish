use clap::{value_t_or_exit, App, Arg, SubCommand};
use shellfish::client::*;
use shellfish::connection::*;
use shellfish::util::runtime::*;
use shellfish::util::socks5;
use std::error::Error;
use std::net::SocketAddr;

#[cfg(feature = "rt-tokio")]
fn main() -> Result<(), Box<dyn Error>> {
    tokio::runtime::Runtime::new()?.block_on(main_async())
}

#[cfg(feature = "rt-async")]
fn main() -> Result<(), Box<dyn Error>> {
    async_std::task::block_on(main_async())
}

async fn main_async() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let config = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::with_name("log")
                .help("Log level")
                .long("log")
                .takes_value(true)
                .default_value("info"),
        )
        .subcommand(
            SubCommand::with_name("socks5")
                .about("SOCKS5 proxy with forwarding over SSH")
                .arg(
                    Arg::with_name("host")
                        .help("SSH host name")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("port")
                        .help("SSH host port")
                        .takes_value(true)
                        .long("port")
                        .default_value("22"),
                )
                .arg(
                    Arg::with_name("user")
                        .help("SSH user name")
                        .long("user")
                        .takes_value(true)
                        .env("LOGNAME"),
                )
                .arg(
                    Arg::with_name("agent")
                        .help("SSH agent socket")
                        .long("agent")
                        .takes_value(true)
                        .env("SSH_AUTH_SOCK"),
                )
                .arg(
                    Arg::with_name("bind")
                        .help("SOCKS5 bind address")
                        .long("bind")
                        .takes_value(true)
                        .default_value("127.0.0.1:1080"),
                ),
        )
        .get_matches();

    // Setup logger
    let filters = value_t_or_exit!(config, "log", String);
    env_logger::Builder::new().parse_filters(&filters).init();

    match config.subcommand() {
        ("socks5", Some(config)) => {
            let user = value_t_or_exit!(config, "user", String);
            let host = value_t_or_exit!(config, "host", String);
            let port = value_t_or_exit!(config, "port", u16);
            let bind = value_t_or_exit!(config, "bind", String);

            let mut pool = ConnectionPool::new(Client::default(), &user, &host, port);
            let listener: TcpListener = TcpListener::bind(&bind).await?;

            loop {
                let (sock, addr) = listener.accept().await?;
                let connection = pool.get().await;
                let _ = spawn(async move {
                    if let Err(e) = serve(sock, addr, connection).await {
                        log::error!("{:?}", e);
                    }
                });
            }
        }
        _ => Ok(()),
    }
}

async fn serve(sock: TcpStream, addr: SocketAddr, conn: Connection) -> Result<(), Box<dyn Error>> {
    let cr = socks5::serve(sock).await?;
    let dh = cr.host().to_string();
    let dp = cr.port();
    let sa = addr.ip();
    let sp = addr.port();
    let chan = conn.open_direct_tcpip(dh, dp, sa, sp).await??;
    let sock = cr.accept(addr).await?;
    chan.interconnect(sock).await?;
    Ok(())
}

#[derive(Clone)]
pub struct ConnectionPool {
    user: String,
    host: String,
    port: u16,
    client: Client,
    connection: Option<Connection>,
}

impl ConnectionPool {
    const MIN_DELAY: u64 = 1;
    const MAX_DELAY: u64 = 300;

    pub fn new(client: Client, user: &str, host: &str, port: u16) -> Self {
        Self {
            user: user.into(),
            host: host.into(),
            port,
            client,
            connection: None,
        }
    }

    pub async fn get(&mut self) -> Connection {
        loop {
            if let Some(c) = &self.connection {
                return c.clone();
            } else {
                self.connection = None;
                self.reconnect().await
            }
        }
    }

    async fn reconnect(&mut self) {
        let u = &self.user;
        let h = &self.host;
        let p = self.port;
        let mut delay = 0;
        loop {
            match self.client.connect(u, h, p, ()).await {
                Err(e) => {
                    log::error!("Connection to {}@{}:{} established failed: {}", u, h, p, e)
                }
                Ok(c) => {
                    log::info!("Connection to {}@{}:{} established", u, h, p);
                    self.connection = Some(c);
                    return;
                }
            };
            delay = std::cmp::min(delay * 2, Self::MAX_DELAY);
            delay = std::cmp::max(delay, Self::MIN_DELAY);
            log::info!("Scheduled reconnect in {} seconds", delay);
            sleep(std::time::Duration::from_secs(delay)).await
        }
    }
}
