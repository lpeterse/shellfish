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
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;
                println!("CONNECTED 1");
                conn.foobar().await.expect("");
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;
                println!("CONNECTED 2");
                conn.foobar().await.expect("");
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;
                println!("CONNECTED 3");
                conn.open_session().await.expect("");
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;
            },
        }
    })
}
