use super::*;

use crate::transport::msg_disconnect::DisconnectReason;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TransportError {
    IoError(std::io::ErrorKind),
    DecoderError,
    ProtocolError,
    BadPacketLength,
    InvalidState,
    InvalidSignature,
    HostKeyUnverifiable,
    MessageIntegrity,
    MessageUnexpected,
    MessageUnimplemented,
    NoCommonServerHostKeyAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorith,
    InactivityTimeout,
    DisconnectByUs(DisconnectReason),
    DisconnectByPeer(DisconnectReason),
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
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
            "DisconnectByUs(DisconnectReason::MAC_ERROR)",
            format!(
                "{:?}",
                TransportError::DisconnectByUs(DisconnectReason::MAC_ERROR)
            )
        );
        assert_eq!(
            "DisconnectByPeer(DisconnectReason::MAC_ERROR)",
            format!(
                "{:?}",
                TransportError::DisconnectByPeer(DisconnectReason::MAC_ERROR)
            )
        );
    }
}
