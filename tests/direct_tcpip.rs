mod agent;
mod host;
mod socket;

use agent::*;
use host::*;
use shellfish::agent::AuthAgent;
use shellfish::connection::{DirectTcpIp, DirectTcpIpRequest};
use shellfish::connection::{Connection, ConnectionConfig, ConnectionHandler, DirectTcpIpParams};
use shellfish::host::HostVerifier;
use shellfish::transport::Transport;
use shellfish::transport::TransportConfig;
use socket::*;
use std::net::Ipv4Addr;
use std::sync::Arc;

const HOST: &'static str = "localhost";
const PORT: u16 = 22;
const SRV: &'static str = "ssh-userauth";

async fn new_transport_pair() -> Result<(Transport, Transport), Box<dyn std::error::Error>> {
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

    Ok((trans1, trans2))
}

async fn new_connection_pair(
    handler: Box<dyn ConnectionHandler>,
) -> Result<(Connection, Connection), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::default();
    let config = Arc::new(config);
    let (trans1, trans2) = new_transport_pair().await?;
    let conn1 = Connection::new(&config, trans1, Box::new(()));
    let conn2 = Connection::new(&config, trans2, handler);
    Ok((conn1, conn2))
}

async fn new_direct_tcpip_pair() -> Result<((Connection, DirectTcpIp), (Connection, DirectTcpIp)), Box<dyn std::error::Error>> {
    struct Handler { sender: tokio::sync::mpsc::UnboundedSender<DirectTcpIp> };
    impl ConnectionHandler for Handler {
        fn on_direct_tcpip_request(&mut self, request: DirectTcpIpRequest) {
            let dti = request.accept();
            let _ = self.sender.send(dti);
        }
    }
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let handler = Box::new(Handler { sender });
    let (conn1, conn2) = new_connection_pair(handler).await?;
    let params = DirectTcpIpParams::new("dst", 23, Ipv4Addr::UNSPECIFIED, 47);
    let dti1 = conn1.open_direct_tcpip(&params).await??;
    let dti2 = receiver.recv().await.unwrap();
    Ok(((conn1, dti1), (conn2, dti2)))
}

#[tokio::test]
async fn test_new_connection() -> Result<(), Box<dyn std::error::Error>> {
    let (conn1, conn2) = new_connection_pair(Box::new(())).await?;
    conn1.check()?;
    conn2.check()?;
    conn1.check_with_keepalive().await?;
    conn2.check_with_keepalive().await?;
    Ok(())
}

#[tokio::test]
async fn test_open_direct_tcpip_ok() -> Result<(), Box<dyn std::error::Error>> {
    let ((conn1, dti1), (conn2, dti2)) = new_direct_tcpip_pair().await?;
    drop(dti1);
    drop(dti2);
    drop(conn1);
    drop(conn2);
    Ok(())
}
