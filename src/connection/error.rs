use super::*;
use crate::transport::TransportError;

#[derive(Clone, Debug)]
pub enum ConnectionError {
    IoError(Arc<std::io::Error>),
    TransportError(TransportError),
    ChannelOpenFailure(ChannelOpenFailure),
    ChannelOpenUnexpected,
    ChannelWindowAdjustUnexpected,
    ChannelWindowAdjustOverflow,
    ChannelIdInvalid,
    ChannelDataUnexpected,
    ChannelEofUnexpected,
    ChannelCloseUnexpected,
    ChannelExtendedDataUnexpected,
    ChannelRequestFailure,
    ChannelFailureUnexpected,
    ChannelSuccessUnexpected,
    ChannelWindowSizeExceeded,
    ChannelWindowSizeOverflow,
    ChannelMaxPacketSizeExceeded,
    ChannelBufferSizeExceeded,
    ChannelTypeMismatch,
    GlobalReplyUnexpected,
    ResourceExhaustion,
    Dropped,
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(Arc::new(e))
    }
}

impl From<TransportError> for ConnectionError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<ChannelOpenFailure> for ConnectionError {
    fn from(e: ChannelOpenFailure) -> Self {
        Self::ChannelOpenFailure(e)
    }
}

impl std::error::Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
