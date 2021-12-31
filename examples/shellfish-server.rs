use shellfish::server::*;
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
    env_logger::init();
    tokio::runtime::Runtime::new()?.block_on(main_async())
}

async fn main_async() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::default();
    let server = Server::new(config);
    server.listen().await?;
    Ok(())
}
