use super::DirectTcpIp;
use super::DirectTcpIpParams;
use super::OpenFailure;
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub struct DirectTcpIpRequest {
    dtcpip: DirectTcpIp,
    params: DirectTcpIpParams,
    reply_tx: Sender<Result<(), OpenFailure>>,
}

impl DirectTcpIpRequest {
    pub(crate) fn new(
        dtcpip: DirectTcpIp,
        params: DirectTcpIpParams,
        reply_tx: Sender<Result<(), OpenFailure>>,
    ) -> Self {
        Self {
            dtcpip,
            params,
            reply_tx,
        }
    }

    pub fn params(&self) -> &DirectTcpIpParams {
        &self.params
    }

    pub fn accept(self) -> DirectTcpIp {
        let _ = self.reply_tx.send(Ok(()));
        self.dtcpip
    }

    pub fn reject(self, e: OpenFailure) {
        let _ = self.reply_tx.send(Err(e));
    }
}
