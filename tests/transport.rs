mod agent;
mod host;
mod socket;

use agent::*;
use host::*;
use shellfish::agent::AuthAgent;
use shellfish::host::HostVerifier;
use shellfish::transport::Transport;
use shellfish::transport::TransportConfig;
use shellfish::transport::TransportError;
use shellfish::transport::DisconnectReason;
use socket::*;
use std::sync::Arc;

const HOST: &'static str = "localhost";
const PORT: u16 = 22;
const SRV: &'static str = "ssh-userauth";

#[tokio::test]
async fn test_connect_ok() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let conf = TransportConfig::default();
    let conf = Arc::new(conf);
    let conf_ = conf.clone();

    let agent = AuthAgentForTesting::new();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let identity = agent.identities().await?[0].0.clone();
    let verifier = HostVerifierForTesting::new(HOST, PORT, &identity);
    let verifier: Arc<dyn HostVerifier> = Arc::new(verifier);

    let task1 = async move { Transport::accept(sock1, &conf, &agent, SRV).await };
    let task2 = async move { Transport::connect(sock2, &conf_, &verifier, HOST, PORT, SRV).await };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let trans1 = task1.await??;
    let trans2 = task2.await??;

    let sid1 = trans1.session_id();
    let sid2 = trans2.session_id();

    assert_eq!(sid1.as_ref(), sid2.as_ref());
    Ok(())
}

#[tokio::test]
async fn test_connect_service_not_available() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let conf = TransportConfig::default();
    let conf = Arc::new(conf);
    let conf_ = conf.clone();

    let agent = AuthAgentForTesting::new();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let identity = agent.identities().await?[0].0.clone();
    let verifier = HostVerifierForTesting::new(HOST, PORT, &identity);
    let verifier: Arc<dyn HostVerifier> = Arc::new(verifier);

    let task1 = async move { Transport::accept(sock1, &conf, &agent, "invalid-service").await };
    let task2 = async move { Transport::connect(sock2, &conf_, &verifier, HOST, PORT, SRV).await };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let err1 = task1.await?.unwrap_err();
    let err2 = task2.await?.unwrap_err();

    match err1 {
        TransportError::InvalidServiceRequest(_) => (),
        e => panic!("{:?}", e)
    }

    match err2 {
        TransportError::DisconnectByPeer(DisconnectReason::SERVICE_NOT_AVAILABLE) => (),
        e => panic!("{:?}", e)
    }

    Ok(())
}

#[tokio::test]
async fn test_connect_agent_no_identities() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let conf = TransportConfig::default();
    let conf = Arc::new(conf);
    let conf_ = conf.clone();

    let agent = AuthAgentForTesting::new().no_identities();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let identity = AuthAgentForTesting::new().identities().await?[0].0.clone();
    let verifier = HostVerifierForTesting::new(HOST, PORT, &identity);
    let verifier: Arc<dyn HostVerifier> = Arc::new(verifier);

    let task1 = async move { Transport::accept(sock1, &conf, &agent, SRV).await };
    let task2 = async move { Transport::connect(sock2, &conf_, &verifier, HOST, PORT, SRV).await };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let err1 = task1.await?.unwrap_err();
    let err2 = task2.await?.unwrap_err();

    match err1 {
        TransportError::NoCommonServerHostKeyAlgorithm => (),
        e => panic!("{:?}", e)
    }

    match err2 {
        TransportError::IoError(_) => (),
        e => panic!("{:?}", e)
    }

    Ok(())
}

#[tokio::test]
async fn test_connect_agent_unable_to_sign() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let conf = TransportConfig::default();
    let conf = Arc::new(conf);
    let conf_ = conf.clone();

    let agent = AuthAgentForTesting::new().unable_to_sign();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let identity = agent.identities().await?[0].0.clone();
    let verifier = HostVerifierForTesting::new(HOST, PORT, &identity);
    let verifier: Arc<dyn HostVerifier> = Arc::new(verifier);

    let task1 = async move { Transport::accept(sock1, &conf, &agent, SRV).await };
    let task2 = async move { Transport::connect(sock2, &conf_, &verifier, HOST, PORT, SRV).await };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let err1 = task1.await?.unwrap_err();
    let err2 = task2.await?.unwrap_err();

    match err1 {
        TransportError::AgentRefusedToSign => (),
        e => panic!("{:?}", e)
    }

    match err2 {
        TransportError::IoError(_) => (),
        e => panic!("{:?}", e)
    }

    Ok(())
}

#[tokio::test]
async fn test_connect_agent_invalid_signature() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let conf = TransportConfig::default();
    let conf = Arc::new(conf);
    let conf_ = conf.clone();

    let agent = AuthAgentForTesting::new().invalid_signature();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let identity = agent.identities().await?[0].0.clone();
    let verifier = HostVerifierForTesting::new(HOST, PORT, &identity);
    let verifier: Arc<dyn HostVerifier> = Arc::new(verifier);

    let task1 = async move { Transport::accept(sock1, &conf, &agent, SRV).await };
    let task2 = async move { Transport::connect(sock2, &conf_, &verifier, HOST, PORT, SRV).await };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let err1 = task1.await?.unwrap_err();
    let err2 = task2.await?.unwrap_err();

    match err1 {
        TransportError::IoError(_) => (),
        e => panic!("{:?}", e)
    }

    match err2 {
        TransportError::InvalidSignature => (),
        e => panic!("{:?}", e)
    }

    Ok(())
}

#[tokio::test]
async fn test_connect_agent_sign_error() -> Result<(), Box<dyn std::error::Error>> {
    let (sock1, sock2) = Socket::new_tcp().await?;

    let conf = TransportConfig::default();
    let conf = Arc::new(conf);
    let conf_ = conf.clone();

    let agent = AuthAgentForTesting::new().sign_error();
    let agent: Arc<dyn AuthAgent> = Arc::new(agent);

    let identity = agent.identities().await?[0].0.clone();
    let verifier = HostVerifierForTesting::new(HOST, PORT, &identity);
    let verifier: Arc<dyn HostVerifier> = Arc::new(verifier);

    let task1 = async move { Transport::accept(sock1, &conf, &agent, SRV).await };
    let task2 = async move { Transport::connect(sock2, &conf_, &verifier, HOST, PORT, SRV).await };

    let task1 = tokio::spawn(task1);
    let task2 = tokio::spawn(task2);

    let err1 = task1.await?.unwrap_err();
    let err2 = task2.await?.unwrap_err();

    match err1 {
        TransportError::AgentError(_) => (),
        e => panic!("{:?}", e)
    }

    match err2 {
        TransportError::IoError(_) => (),
        e => panic!("{:?}", e)
    }

    Ok(())
}
