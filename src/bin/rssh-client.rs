use rssh::client::*;
use rssh::service::connection::*;
use std::future::Future;
use env_logger;
use log::*;

fn main() {
    env_logger::init();
    info!("shdksjhda");

    futures::executor::block_on(async move {
        let mut client = Client::default();
        //*client.user_name() = None;
        let conn = client
            .connect("localhost:22")
            .await;
        match conn {
            Err(e) => println!("{:?}", e),
            Ok(mut conn) => {
                loop {
                    async_std::task::sleep(std::time::Duration::from_secs(5)).await;
                    log::error!("DEBUG");
                    conn.debug("QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ".into()).await.expect("");
                    async_std::task::sleep(std::time::Duration::from_secs(5)).await;
                    log::error!("OPEN SESSION");
                    conn.open_session().await.expect("");
                    log::info!("SESSION OPEN");
                }
                conn.disconnect().await.expect("");
                println!("DISCONNE");
                //conn.foobar().await.expect("");
                //async_std::task::sleep(std::time::Duration::from_secs(1)).await;
                //println!("CONNECTED 3");
                async_std::task::sleep(std::time::Duration::from_secs(10)).await;
            },
        }
    })
}
