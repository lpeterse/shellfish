use async_std::task::block_on;
use rssh::client::*;

fn main() {
    block_on(async move {
        let client = Client::new(Config {});
        let conn = client.connect("google.com:80").await;

        async_std::task::sleep(std::time::Duration::from_secs(5)).await;
        println!("FOO");
    })
}