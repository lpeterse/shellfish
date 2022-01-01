mod agent;
mod host;
mod socket;

use agent::*;
use host::*;
use shellfish::agent::AuthAgent;
use shellfish::host::HostVerifier;
use shellfish::transport::Transport;
use shellfish::transport::TransportConfig;
use socket::*;
use std::sync::Arc;

#[tokio::test]
async fn test_kex_01() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let config = TransportConfig::default();
    let config = Arc::new(config);
    let config_ = config.clone();

    let agent = AuthAgentForTesting::new();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let host_name = "localhost";
    let host_port = 22;
    let host_identity = agent.identities().await?[0].0.clone();
    let host_verifier = HostVerifierForTesting::new(host_name, host_port, &host_identity);
    let host_verifier: Arc<dyn HostVerifier> = Arc::new(host_verifier);

    let task1 = async move { Transport::accept(&config, sock1, &agent).await };
    let task2 = async move {
        Transport::connect(&config_, sock2, host_name, host_port, &host_verifier).await
    };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let trans1 = task1.await??;
    let trans2 = task2.await??;

    let sid1 = trans1.session_id();
    let sid2 = trans2.session_id();

    assert_eq!(sid1.as_ref(), sid2.as_ref());
    Ok(())
}
