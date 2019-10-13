use super::*;

use crate::transport::msg_disconnect::Reason;

#[derive(Debug, Copy, Clone)]
pub enum TransportError {
    IoError(std::io::ErrorKind),
    KexError(KexError),
    DecoderError,
    BadPacketLength,
    MessageIntegrity,
    MessageUnexpected,
    MessageUnimplemented,
    DisconnectByUs(Reason),
    DisconnectByPeer(Reason),
    InactivityTimeout,
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
    }
}

impl From<KexError> for TransportError {
    fn from(e: KexError) -> Self {
        Self::KexError(e)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_kex_error_01() {
        let x: TransportError = KexError::InvalidSignature.into();
        match x {
            TransportError::KexError(KexError::InvalidSignature) => (),
            _ => panic!(""),
        }
    }

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
