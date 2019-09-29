use super::*;
use crate::requestable;

use std::sync::PoisonError;

#[derive(Copy, Clone, Debug)]
pub enum ConnectionError {
    Canceled,
    PoisonError,
    CommandStreamExhausted,
    TransportStreamExhausted,
    InvalidChannelId,
    InvalidChannelState,
    ChannelOpenFailure(ChannelOpenFailureReason),
    ChannelRequestFailure,
    ChannelFailureUnexpected,
    ChannelSuccessUnexpected,
    ChannelWindowSizeUnderrun,
    RequestError(requestable::Error),
    TransportError(TransportError),
}

impl <T> From<PoisonError<T>> for ConnectionError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

impl From<TransportError> for ConnectionError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<futures::channel::oneshot::Canceled> for ConnectionError {
    fn from(_: futures::channel::oneshot::Canceled) -> Self {
        Self::Canceled
    }
}

impl From<requestable::Error> for ConnectionError {
    fn from(e: requestable::Error) -> Self {
        Self::RequestError(e)
    }
}

impl From<ChannelOpenFailureReason> for ConnectionError {
    fn from(e: ChannelOpenFailureReason) -> Self {
        Self::ChannelOpenFailure(e)
    }
}
