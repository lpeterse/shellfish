use shellfish::server::*;
use std::error::Error;

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
