use super::super::state::ChannelState;
use super::super::OpenFailure;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct SessionServerState;

impl SessionServerState {
    pub fn new(
        lid: u32,
        mbs: u32,
        lmps: u32,
        rid: u32,
        rws: u32,
        rmps: u32,
        resp: oneshot::Receiver<Result<(), OpenFailure>>,
    ) -> Self {
        todo!()
    }
}

impl ChannelState for Arc<Mutex<SessionServerState>> {
    fn poll_with_transport(
        &mut self,
        cx: &mut std::task::Context,
        t: &mut crate::transport::GenericTransport,
    ) -> std::task::Poll<Result<bool, crate::connection::ConnectionError>> {
        todo!()
    }
}
