use rssh::agent::*;
use env_logger;
use rssh::keys::*;
use rssh::algorithm::*;

fn main() {
    env_logger::init();

    futures::executor::block_on(async move {
        let mut agent = Agent::new_env().expect("");
        let digest: [u8;3] = [0,1,2];
        
        for (key,_) in agent.identities().await.expect("identities") {
            println!("KEY {:?}", key);
            match key {
                PublicKey::Ed25519PublicKey(key) => {
                    println!("SIGN");
                    let signature = agent.sign::<SshEd25519>(key, &digest, 0).await;
                    println!("{:?}", signature);
                },
                key => println!("ignore {:?}", key),
            }
        }
    });
}
