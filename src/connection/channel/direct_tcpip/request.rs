use super::DirectTcpIp;
use super::DirectTcpIpParams;
use super::OpenFailure;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct DirectTcpIpRequest {
    pub params: DirectTcpIpParams,
    pub channel: DirectTcpIp,
    pub response: oneshot::Sender<Result<(), OpenFailure>>,
}

impl DirectTcpIpRequest {
    pub fn params(&self) -> &DirectTcpIpParams {
        &self.params
    }

    pub fn accept(self) -> DirectTcpIp {
        let _ = self.response.send(Ok(()));
        self.channel
    }

    pub fn reject(self, fail: OpenFailure) {
        let _ = self.response.send(Err(fail));
    }
}
