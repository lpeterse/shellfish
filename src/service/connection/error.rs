use super::*;

#[derive(Debug)]
pub enum ConnectionError {
    ConnectionLost,
    CommandStreamExhausted,
    TransportStreamExhausted,
    InvalidChannelId,
    InvalidChannelState,
    TransportError(TransportError),
    ChannelOpenFailure(ChannelOpenFailure),
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
