use rssh::client::*;
use rssh::service::connection::*;
use std::future::Future;
use env_logger;
use log::*;

fn main() {
    env_logger::init();
    info!("shdksjhda");

    futures::executor::block_on(async move {
        {
            let c = Connection2::new();
            async_std::task::sleep(std::time::Duration::from_secs(6)).await;
            let x = c;
        }
        log::error!("FINISHED");
    });

    futures::executor::block_on(async move {
        let client = Client::new(Config {});
        let conn = client
            .connect("localhost:22")
            .await;
        match conn {
            Err(e) => println!("{:?}", e),
            Ok(mut conn) => match conn.channel().await {
                Err(e) => println!("{:?}", e),
                Ok(_) => (),
            }
        }
    })
}