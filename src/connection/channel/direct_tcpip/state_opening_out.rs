use super::super::super::error::ConnectionError;
use super::super::open_failure::OpenFailure;
use super::super::ChannelState;
use super::state::State;
use super::{DirectTcpIp, DirectTcpIpParams};
use crate::connection::channel::PollResult;
use crate::ready;
use crate::transport::Transport;
use crate::util::codec::SshCodec;
use std::task::Context;
use std::task::Poll;
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub(crate) struct StateOpeningOutbound {
    state: State,
    params: Option<DirectTcpIpParams>,
    reply_tx: Sender<Result<DirectTcpIp, OpenFailure>>,
}

impl StateOpeningOutbound {
    pub fn new(
        state: State,
        params: DirectTcpIpParams,
        reply_tx: Sender<Result<DirectTcpIp, OpenFailure>>,
    ) -> Self {
        Self {
            state,
            params: Some(params),
            reply_tx,
        }
    }
}

impl ChannelState for StateOpeningOutbound {
    fn on_open_confirmation(
        self: Box<Self>,
        rid: u32,
        rws: u32,
        rps: u32,
    ) -> Result<Box<dyn ChannelState>, ConnectionError> {
        self.state.set_open_confirmation(rid, rws, rps);
        let t = DirectTcpIp::new(self.state.clone());
        let _ = self.reply_tx.send(Ok(t));
        let c = Box::new(self.state);
        Ok(c)
    }

    fn on_open_failure(self: Box<Self>, e: OpenFailure) -> Result<(), ConnectionError> {
        let _ = self.reply_tx.send(Err(e));
        Ok(())
    }

    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<PollResult, ConnectionError>> {
        if let Some(params) = &self.params {
            let prm = SshCodec::encode(params)?;
            let msg = self.state.msg_open(prm);
            ready!(t.poll_send(cx, &msg))?;
            self.params = None;
        }
        Poll::Ready(Ok(PollResult::Noop))
    }
}
