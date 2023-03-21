use super::super::super::error::ConnectionError;
use super::super::open_failure::OpenFailure;
use super::super::ChannelState;
use super::state::State;
use crate::connection::channel::PollResult;
use crate::transport::Transport;
use std::future::Future;
use std::pin::Pin;
use std::task::{Poll, Context, ready};
use tokio::sync::oneshot::Receiver;

/// This is the state in which a MSG_CHANNEL_OPEN message has been received and is open
/// for accept/reject decision by the local handler function.
///
/// As soon as the local handler function returns its decision through the channel (or drops the
/// channel) _and_ the MSG_CHANNEL_OPEN_(SUCCESS|FAILURE) message has been sent, this object
/// gets dropped and replaces itself with [State] which means the channel is now open for use.
#[derive(Debug)]
pub(crate) struct StateOpeningInbound {
    state: State,
    reply_rx: Receiver<Result<(), OpenFailure>>,
    reply_rxd: Option<Result<(), OpenFailure>>,
}

impl StateOpeningInbound {
    pub fn new(state: State, reply_rx: Receiver<Result<(), OpenFailure>>) -> Self {
        Self {
            state,
            reply_rx,
            reply_rxd: None,
        }
    }
}

impl ChannelState for StateOpeningInbound {
    fn poll(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<PollResult, ConnectionError>> {
        loop {
            match self.reply_rxd {
                Some(Ok(_)) => {
                    let msg = self.state.msg_open_confirmation();
                    ready!(t.poll_send(cx, &msg)?);
                    let bchan = Box::new(self.state.clone());
                    return Poll::Ready(Ok(PollResult::Replace(bchan)));
                }
                Some(Err(e)) => {
                    let msg = self.state.msg_open_failure(e);
                    ready!(t.poll_send(cx, &msg)?);
                    return Poll::Ready(Ok(PollResult::Closed));
                }
                None => {
                    let e = OpenFailure::ADMINISTRATIVELY_PROHIBITED;
                    match Pin::new(&mut self.reply_rx).poll(cx) {
                        Poll::Pending => return Poll::Ready(Ok(PollResult::Noop)),
                        Poll::Ready(Ok(r)) => self.reply_rxd = Some(r),
                        Poll::Ready(Err(_)) => self.reply_rxd = Some(Err(e)),
                    }
                }
            }
        }
    }

    fn is_closed(&self) -> bool {
        panic!()
    }
}
