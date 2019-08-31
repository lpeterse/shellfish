use async_std::task::block_on;
use rssh::client::*;

fn main() {
    block_on(async move {
        let client = Client::new(Config {});
        let conn = client.connect("localhost:22").await;
    })
}