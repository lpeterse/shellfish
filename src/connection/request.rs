use super::channel::DirectTcpIp;
use super::channel::DirectTcpIpParams;
use super::channel::OpenFailure;
use super::channel::SessionClient;
use tokio::sync::oneshot::{channel, Receiver, Sender};

#[derive(Debug)]
pub enum Request {
    Global {
        name: &'static str,
        data: Vec<u8>,
        reply: Option<Sender<Result<Vec<u8>, ()>>>,
    },
    OpenSession {
        reply: Sender<Result<SessionClient, OpenFailure>>,
    },
    OpenDirectTcpIp {
        params: DirectTcpIpParams,
        reply: Sender<Result<DirectTcpIp, OpenFailure>>,
    },
}

impl Request {
    pub fn open_session() -> (Self, Receiver<Result<SessionClient, OpenFailure>>) {
        let (tx, rx) = channel();
        let self_ = Self::OpenSession { reply: tx };
        (self_, rx)
    }

    pub fn open_direct_tcpip(
        params: DirectTcpIpParams,
    ) -> (Self, Receiver<Result<DirectTcpIp, OpenFailure>>) {
        let (tx, rx) = channel();
        let self_ = Self::OpenDirectTcpIp { params, reply: tx };
        (self_, rx)
    }
}
