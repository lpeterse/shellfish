use rssh::agent::*;

#[tokio::main]
async fn main() {
    let agent = LocalAgent::new();
    let ids = agent.request_identities().await;
    println!("{:?}", ids)
}