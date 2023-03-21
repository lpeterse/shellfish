use clap::*;
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[derive(Subcommand)]
enum Commands {
    Socks5 {
        #[arg(short = 'H', long)]
        host: String,
        #[arg(short, long, default_value_t = 22)]
        port: u16,
        #[arg(short, long, env = "LOGNAME")]
        user: String,
        #[arg(short, long, default_value = "[::]:1080")]
        bind: std::net::SocketAddr,
        #[arg(short, long, env = "SSH_AUTH_SOCK")]
        agent: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    tokio::runtime::Runtime::new()?.block_on(main_async())
}

async fn main_async() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Setup logger
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    match cli.command {
        Commands::Socks5 { host, port, user, bind, agent: _ } => {
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
                match client.connect(&u, &h, port, Box::new(())).await {
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
