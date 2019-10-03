use super::*;

#[derive(Copy, Clone, Debug)]
pub enum ConnectionError {
    Terminated,
    IoError(std::io::ErrorKind),
    TransportError(TransportError),
    ChannelOpenFailure(ChannelOpenFailure),
    ChannelIdInvalid,
    ChannelRequestFailure,
    ChannelFailureUnexpected,
    ChannelSuccessUnexpected,
    ChannelWindowSizeUnderrun,
    RequestSenderDropped,
    RequestReceiverDropped,
    RequestUnexpectedResponse,
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
