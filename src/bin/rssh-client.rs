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
        *client.user_name() = None;
        let conn = client
            .connect("localhost:22")
            .await;
        match conn {
            Err(e) => println!("{:?}", e),
            Ok(conn) => println!("connected"),
        }
    })
}
