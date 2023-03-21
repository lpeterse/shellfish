use super::channel::{OpenFailure, RequestFailure};
use crate::{transport::TransportError, util::codec::SshCodecError};
use std::sync::Arc;
use tokio::sync::watch;

pub type ConnectionErrorWatch = watch::Receiver<Option<Arc<ConnectionError>>>;

#[derive(Clone, Debug)]
pub enum ConnectionError {
    IoError(std::sync::Arc<std::io::Error>),
    CodecError(SshCodecError),
    TransportError(TransportError),
    ChannelBufferSizeExceeded,
    ChannelCloseUnexpected,
    ChannelDataUnexpected,
    ChannelEofUnexpected,
    ChannelExtendedDataUnexpected,
    ChannelFailureUnexpected,
    ChannelInvalid,
    ChannelInvalidState,
    ChannelOpenConfirmationUnexpected,
    ChannelOpenFailure(OpenFailure),
    ChannelOpenFailureUnexpected,
    ChannelOpenUnexpected,
    ChannelPacketSizeExceeded,
    ChannelPacketSizeInvalid,
    ChannelPtyRejected,
    ChannelPtyReqUnexpected,
    ChannelRequestFailure,
    ChannelRequestUnexpected,
    ChannelSuccessUnexpected,
    ChannelTypeMismatch,
    ChannelWindowAdjustOverflow,
    ChannelWindowAdjustUnexpected,
    ChannelWindowSizeExceeded,
    ChannelWindowSizeOverflow,
    GlobalReplyUnexpected,
    ResourceExhaustion,
    Dropped,
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(std::sync::Arc::new(e))
    }
}

impl From<SshCodecError> for ConnectionError {
    fn from(e: SshCodecError) -> Self {
        Self::CodecError(e)
    }
}

impl From<TransportError> for ConnectionError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<OpenFailure> for ConnectionError {
    fn from(e: OpenFailure) -> Self {
        Self::ChannelOpenFailure(e)
    }
}

impl <T> From<RequestFailure<T>> for ConnectionError {
    fn from(_: RequestFailure<T>) -> Self {
        Self::ChannelRequestFailure
    }
}

impl std::error::Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
