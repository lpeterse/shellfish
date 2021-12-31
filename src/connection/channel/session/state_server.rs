use crate::connection::channel::PollResult;

use super::super::super::config::ConnectionConfig;
use super::super::super::error::ConnectionError;
use super::super::super::msg::*;
use super::super::ChannelState;
use super::super::OpenFailure;
use super::server::SessionRequest;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;

pub(crate) fn open(
    config: &ConnectionConfig,
    msg: &MsgChannelOpen,
    lid: u32,
) -> Result<(Box<dyn ChannelState>, SessionRequest), ConnectionError> {
    /*
    let cst = SessionServerState::new(lid, lws, lps, rid, rws, rps, r);
    let cst = Arc::new(Mutex::new(cst));
    let req = SessionRequest {
        chan: SessionServer(cst.clone()),
        resp: s,
    };
    self.handler.on_session_request(req);
    self.channels[lid as usize] = Some(Box::new(cst));
    */
    todo!()
}

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
    ) -> std::task::Poll<Result<PollResult, crate::connection::ConnectionError>> {
        todo!()
    }
}
