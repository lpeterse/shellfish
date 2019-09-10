use rssh::agent::*;
use env_logger;

fn main() {
    env_logger::init();

    futures::executor::block_on(async move {
        let mut agent = Agent::new_env().expect("");
        let identities = agent.identities().await.expect("identities");
        println!("{:?}", &identities);
        let digest = [0,1,2,3];
        let identity = identities[0].0.clone();
        let signature = agent.sign(identity, &digest, 0).await;
        println!("{:?}", signature);
    });
}
