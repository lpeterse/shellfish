use super::*;
use crate::transport::TransportError;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ConnectionError {
    IoError(std::io::ErrorKind),
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
    Unknown,
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

impl From<Result<DisconnectReason, ConnectionError>> for ConnectionError {
    fn from(e: Result<DisconnectReason, ConnectionError>) -> Self {
        match e {
            Ok(reason) => ConnectionError::TransportError(TransportError::DisconnectByPeer(reason)),
            Err(e) => e,
        }
    }
}

impl std::error::Error for ConnectionError {}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        assert_eq!("Unknown", format!("{:?}", ConnectionError::Unknown));
        assert_eq!(
            "IoError(Other)",
            format!("{:?}", ConnectionError::IoError(std::io::ErrorKind::Other))
        );
        assert_eq!(
            "TransportError(BadPacketLength)",
            format!(
                "{:?}",
                ConnectionError::TransportError(TransportError::BadPacketLength)
            )
        );
        assert_eq!(
            "ChannelIdInvalid",
            format!("{:?}", ConnectionError::ChannelIdInvalid)
        );
        assert_eq!(
            "ChannelRequestFailure",
            format!("{:?}", ConnectionError::ChannelRequestFailure)
        );
        assert_eq!(
            "ChannelFailureUnexpected",
            format!("{:?}", ConnectionError::ChannelFailureUnexpected)
        );
        assert_eq!(
            "ChannelSuccessUnexpected",
            format!("{:?}", ConnectionError::ChannelSuccessUnexpected)
        );
        assert_eq!(
            "ChannelWindowSizeExceeded",
            format!("{:?}", ConnectionError::ChannelWindowSizeExceeded)
        );
        assert_eq!(
            "ChannelWindowSizeOverflow",
            format!("{:?}", ConnectionError::ChannelWindowSizeOverflow)
        );
    }

    #[test]
    fn test_from_io_error_01() {
        match std::io::Error::new(std::io::ErrorKind::Other, "").into() {
            ConnectionError::IoError(_) => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_from_transport_error_01() {
        match TransportError::BadPacketLength.into() {
            ConnectionError::TransportError(_) => (),
            _ => panic!(""),
        }
    }
}
