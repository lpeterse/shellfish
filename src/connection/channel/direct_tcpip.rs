mod params;
mod request;
mod state;
mod state_opening_in;
mod state_opening_out;

use crate::connection::ConnectionConfig;
use crate::connection::ConnectionError;
use crate::connection::msg::MsgChannelOpen;
use crate::util::codec::SshCodec;

pub use self::params::DirectTcpIpParams;
pub use self::request::DirectTcpIpRequest;

use self::state::State;
use self::state_opening_in::StateOpeningInbound;
use self::state_opening_out::StateOpeningOutbound;
use super::Channel;
use super::ChannelState;
use super::OpenFailure;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::oneshot::Sender;
use tokio::sync::oneshot::channel;

#[derive(Debug)]
pub struct DirectTcpIp(State);

impl DirectTcpIp {
    pub(crate) fn new(state: State) -> Self {
        Self(state)
    }

    pub (crate) fn open_in(
        config: &ConnectionConfig,
        msg: &MsgChannelOpen,
        lid: u32,
    ) -> Result<(Box<dyn ChannelState>, DirectTcpIpRequest), ConnectionError> {
        let (tx, rx) = channel();
        let lbs = config.channel_max_buffer_size;
        let lps = config.channel_max_packet_size;
        let rid = msg.sender_channel;
        let rws = msg.initial_window_size;
        let rps = msg.maximum_packet_size;
        let st1 = State::new(lid, lbs, lps, rid, rws, rps);
        let dti = DirectTcpIp::new(st1.clone());
        let prm = SshCodec::decode(&msg.data)?;
        let req = DirectTcpIpRequest::new(dti, prm, tx);
        let bch = StateOpeningInbound::new(st1, rx);
        let bch = Box::new(bch);
        Ok((bch, req))
    }
    
    pub (crate) fn open_out(
        config: &ConnectionConfig,
        lid: u32,
        reply_tx: Sender<Result<DirectTcpIp, OpenFailure>>,
        params: DirectTcpIpParams,
    ) -> Result<Box<dyn ChannelState>, ConnectionError> {
        let lbs = config.channel_max_buffer_size;
        let lps = config.channel_max_packet_size;
        let st1 = State::new(lid, lbs, lps, 0, 0, 0);
        let bch = StateOpeningOutbound::new(st1, params, reply_tx);
        let bch = Box::new(bch);
        Ok(bch)
    }
}

impl Channel for DirectTcpIp {
    const NAME: &'static str = "direct-tcpip";
}

/// Dropping initiates the channel close procedure.
///
/// Pending data will be transmitted before sending an `SSH_MSG_CHANNEL_CLOSE`.
/// The channel gets freed after `SSH_MSG_CHANNEL_CLOSE` has been sent _and_
/// received. Of course, the [drop] call itself does not block, but the
/// processing is performed by the internal connection task as long as the
/// connection is alive.
impl Drop for DirectTcpIp {
    fn drop(&mut self) {
        self.0.close()
    }
}

impl AsyncRead for DirectTcpIp {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for DirectTcpIp {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}
