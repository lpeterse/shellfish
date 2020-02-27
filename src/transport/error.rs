use super::*;

use crate::algorithm::authentication::*;
use crate::transport::msg_disconnect::Reason;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TransportError {
    IoError(std::io::ErrorKind),
    IdentificationTimeout,
    DecoderError,
    ProtocolError,
    BadPacketLength,
    InvalidSignature,
    HostKeyUnverifiable,
    MessageIntegrity,
    MessageUnexpected,
    MessageUnimplemented,
    NoCommonServerAuthenticationAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorith,
    InactivityTimeout,
    DisconnectByUs(Reason),
    DisconnectByPeer(Reason),
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
    }
}

impl From<SignatureError> for TransportError {
    fn from(_: SignatureError) -> Self {
        Self::InvalidSignature
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_io_error_01() {
        let x: TransportError = std::io::Error::new(std::io::ErrorKind::Other, "").into();
        match x {
            TransportError::IoError(std::io::ErrorKind::Other) => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_debug_01() {
        assert_eq!(
            "DisconnectByUs(Reason::MAC_ERROR)",
            format!("{:?}", TransportError::DisconnectByUs(Reason::MAC_ERROR))
        );
        assert_eq!(
            "DisconnectByPeer(Reason::MAC_ERROR)",
            format!("{:?}", TransportError::DisconnectByPeer(Reason::MAC_ERROR))
        );
    }
}
