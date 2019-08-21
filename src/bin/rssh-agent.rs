
use tokio::prelude::*;
use rssh::agent::*;

fn main() {
    let agent = LocalAgent::new();
    let future = agent.request_identities()
        .map(|ids| println!("{:?}", ids))
        .map_err(|e| println!("{:?}", e));
    tokio::run(future);
}