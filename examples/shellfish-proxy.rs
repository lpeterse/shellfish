use clap::{value_t_or_exit, App, AppSettings, Arg, SubCommand};
use shellfish::client::*;
use shellfish::connection::*;
use shellfish::util::socks5;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::watch;
use tokio::time::sleep;

fn main() -> Result<(), Box<dyn Error>> {
    tokio::runtime::Runtime::new()?.block_on(main_async())
}

async fn main_async() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let config = App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::ArgRequiredElseHelp)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::with_name("verbosity")
                .help("Verbosity")
                .short('v')
                .multiple(true),
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
    let log_level = match config.occurrences_of("verbosity") {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    println!("{:?}", log_level);
    env_logger::Builder::new()
        .parse_default_env()
        .filter_level(log_level)
        .init();

    match config.subcommand() {
        Some(("socks5", config)) => {
            let user = value_t_or_exit!(config, "user", String);
            let host = value_t_or_exit!(config, "host", String);
            let port = value_t_or_exit!(config, "port", u16);
            let bind = value_t_or_exit!(config, "bind", String);

            let pool = ConnectionPool::new(Client::default(), &user, &host, port);
            let listener: TcpListener = TcpListener::bind(&bind).await?;
            log::info!("Started listening on {}", bind);

            loop {
                let (sock, addr) = listener.accept().await?;
                let mut pool_ = pool.clone();
                let _ = spawn(async move {
                    match pool_.get().await {
                        Err(e) => log::error!("{:?}", e),
                        Ok(cn) => {
                            if let Err(e) = serve(sock, addr, cn).await {
                                log::error!("{:?}", e);
                            }
                        }
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
    let rq = DirectTcpIpParams {
        dst_host: dh,
        dst_port: dp,
        src_addr: sa,
        src_port: sp,
    };
    let mut chan = conn.open_direct_tcpip(&rq).await??;
    let mut sock = cr.accept(addr).await?;
    tokio::io::copy_bidirectional(&mut sock, &mut chan).await?;
    Ok(())
}

#[derive(Clone)]
pub struct ConnectionPool {
    channel: watch::Receiver<Option<Connection>>,
}

impl ConnectionPool {
    const MIN_DELAY: u64 = 1;
    const MAX_DELAY: u64 = 300;

    pub fn new(client: Client, user: &str, host: &str, port: u16) -> Self {
        let (s, r) = watch::channel(None);
        let u = String::from(user);
        let h = String::from(host);
        let p = port;
        spawn(async move {
            let mut delay = 1;
            loop {
                match client.connect(&u, &h, port, |_| Box::new(())).await {
                    Err(e) => {
                        log::error!("Connection to {}@{}:{} failed: {}", u, h, p, e)
                    }
                    Ok(mut c) => {
                        log::info!("Connection to {}@{}:{} established", u, h, p);
                        if s.send(Some(c.clone())).is_err() {
                            return;
                        }
                        c.closed().await;
                        let e = c.check().unwrap_err();
                        log::info!("Connection to {}@{}:{} lost: {}", u, h, p, e);
                    }
                };
                delay = std::cmp::min(delay * 2, Self::MAX_DELAY);
                delay = std::cmp::max(delay, Self::MIN_DELAY);
                log::info!("Scheduled reconnect in {} seconds", delay);
                sleep(std::time::Duration::from_secs(delay)).await
            }
        });
        Self { channel: r }
    }

    pub async fn get(&mut self) -> Result<Connection, Box<dyn Error + Send + Sync>> {
        loop {
            if let Some(x) = self.channel.borrow().as_ref() {
                return Ok(x.clone());
            }
            self.channel.changed().await?;
        }
    }
}
