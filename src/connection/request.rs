use super::channel::DirectTcpIp;
use super::channel::OpenFailure;
use super::channel::Session;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ConnectionRequest {
    Global {
        name: &'static str,
        data: Vec<u8>,
        reply: Option<oneshot::Sender<Result<Vec<u8>, ()>>>,
    },
    OpenSession {
        reply: oneshot::Sender<Result<Session, OpenFailure>>,
    },
    OpenDirectTcpIp {
        data: Vec<u8>, // FIXME
        reply: oneshot::Sender<Result<DirectTcpIp, OpenFailure>>,
    },
}
