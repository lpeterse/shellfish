use super::super::ConnectionError;
use super::open_failure::OpenFailure;
use crate::transport::GenericTransport;
use std::sync::Arc;
use std::task::{Context, Poll};

/// A collection of event handlers that need to be supported by every channel.
///
/// Certain methods contain default implementations that throw a corresponding error as not all
/// channels support all channel messages (like extended data).
pub trait ChannelState: Send + Sync + 'static {
    /// A channel open request has been accepted by the peer.
    ///
    /// The method consumes the half-open channel state and shall return a new channel state for
    /// the now open channel.
    fn on_open_confirmation(
        self: Box<Self>,
        rid: u32,
        rws: u32,
        rps: u32,
    ) -> Result<Box<dyn ChannelState>, ConnectionError> {
        drop(rid);
        drop(rws);
        drop(rps);
        Err(ConnectionError::ChannelOpenConfirmationUnexpected)
    }
    /// A channel open request has been rejected by the peer.
    ///
    /// The method consumes the half-open channel state and shall drop it after having dispatched
    /// the channel open failure.
    fn on_open_failure(self: Box<Self>, e: OpenFailure) -> Result<(), ConnectionError> {
        drop(e);
        Err(ConnectionError::OpenFailureUnexpected)
    }
    /// The peer sent data for this channel.
    fn on_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        drop(data);
        Err(ConnectionError::ChannelDataUnexpected)
    }
    /// The peer sent extended data for this channel.
    fn on_ext_data(&mut self, typ: u32, data: &[u8]) -> Result<(), ConnectionError> {
        drop(typ);
        drop(data);
        Err(ConnectionError::ChannelExtendedDataUnexpected)
    }
    /// The peer sent a window adjust message.
    fn on_window_adjust(&mut self, bytes: u32) -> Result<(), ConnectionError> {
        drop(bytes);
        Err(ConnectionError::ChannelWindowAdjustUnexpected)
    }
    /// The peer sent a channel request.
    fn on_request(
        &mut self,
        name: &str,
        data: &[u8],
        want_reply: bool,
    ) -> Result<(), ConnectionError> {
        drop(name);
        drop(data);
        drop(want_reply);
        Err(ConnectionError::ChannelRequestUnexpected)
    }
    /// The peer accepted a channel request for which a reply was requested.
    fn on_success(&mut self) -> Result<(), ConnectionError> {
        Err(ConnectionError::ChannelSuccessUnexpected)
    }
    /// The peer rejected a channel request for which a reply was requested.
    fn on_failure(&mut self) -> Result<(), ConnectionError> {
        Err(ConnectionError::ChannelFailureUnexpected)
    }
    /// The peer sent EOF.
    ///
    /// The peer must not sent more data (or extended data) on this channel.
    fn on_eof(&mut self) -> Result<(), ConnectionError> {
        Err(ConnectionError::ChannelEofUnexpected)
    }
    /// The peer wants to close the channel.
    ///
    /// The peer must not sent any more messages on this channel. The local end shall try to send
    /// any pending data and then also send a close message (if not already sent).
    fn on_close(&mut self) -> Result<(), ConnectionError> {
        Err(ConnectionError::ChannelCloseUnexpected)
    }
    /// A connection error occured.
    ///
    /// The handler is supposed to dispatch the error and die.
    fn on_error(self: Box<Self>, e: &Arc<ConnectionError>) {
        drop(e)
    }
    /// Poll the handler.
    ///
    /// The handler shall perform all ready actions and register all relevant events with the
    /// supplied context. It shall return `true` if the channel got closed and shall be removed.
    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut GenericTransport,
    ) -> Poll<Result<bool, ConnectionError>>;
}
