use async_std::task::block_on;
use std::error::Error;

use rssh::server::*;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    block_on(async move {
        let config = ServerConfig::default();
        let server = Server::listen(config).await?;
        loop {
            let (_, addr) = server.accept().await?;
            log::warn!("New connection from {}", addr);
        }
    })
}
