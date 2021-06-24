use crate::transport::TransportError;
use super::channel::OpenFailure;
use tokio::sync::watch;
use std::sync::Arc;

pub type ConnectionErrorWatch = watch::Receiver<Option<Arc<ConnectionError>>>;

#[derive(Clone, Debug)]
pub enum ConnectionError {
    IoError(std::sync::Arc<std::io::Error>),
    TransportError(TransportError),
    OpenFailure(OpenFailure),
    ChannelOpenUnexpected,
    ChannelOpenConfirmationUnexpected,
    OpenFailureUnexpected,
    ChannelWindowAdjustUnexpected,
    ChannelWindowAdjustOverflow,
    ChannelIdInvalid,
    ChannelDataUnexpected,
    ChannelEofUnexpected,
    ChannelCloseUnexpected,
    ChannelExtendedDataUnexpected,
    ChannelRequestFailure,
    ChannelRequestUnexpected,
    ChannelFailureUnexpected,
    ChannelSuccessUnexpected,
    ChannelWindowSizeExceeded,
    ChannelWindowSizeOverflow,
    ChannelMaxPacketSizeExceeded,
    ChannelBufferSizeExceeded,
    ChannelTypeMismatch,
    ChannelPtyRejected,
    GlobalReplyUnexpected,
    ResourceExhaustion,
    Dropped,
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(std::sync::Arc::new(e))
    }
}

impl From<TransportError> for ConnectionError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<OpenFailure> for ConnectionError {
    fn from(e: OpenFailure) -> Self {
        Self::OpenFailure(e)
    }
}

impl std::error::Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
