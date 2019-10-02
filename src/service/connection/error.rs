use super::*;

use std::sync::PoisonError;

#[derive(Copy, Clone, Debug)]
pub enum ConnectionError {
    Terminated,
    IoError(std::io::ErrorKind),
    TransportError(TransportError),
    PoisonError,
    CommandStreamExhausted,
    TransportStreamExhausted,
    InvalidChannelId,
    InvalidChannelState,
    ChannelOpenFailure(ChannelOpenFailure),
    ChannelRequestFailure,
    ChannelFailureUnexpected,
    ChannelSuccessUnexpected,
    ChannelWindowSizeUnderrun,
    RequestSenderDropped,
    RequestReceiverDropped,
    RequestUnexpectedResponse,
}

impl <T> From<PoisonError<T>> for ConnectionError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
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
