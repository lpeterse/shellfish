use super::*;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum TransportError {
    IoError(Arc<std::io::Error>),
    InvalidState,
    InvalidPacket,
    InvalidPacketLength,
    InvalidEncoding,
    InvalidEncryption,
    InvalidSignature,
    InvalidIdentification,
    InvalidIdentity(HostVerificationError),
    NoCommonServerHostKeyAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorithm,
    DisconnectByUs(DisconnectReason),
    DisconnectByPeer(DisconnectReason),
}

impl Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(Arc::new(e))
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "{}", e),
            Self::InvalidState => write!(f, "Invalid state (protocol error)"),
            Self::InvalidPacket => write!(f, "Invalid packet structure"),
            Self::InvalidPacketLength => write!(f, "Invalid packet length"),
            Self::InvalidEncoding => write!(f, "Invalid encoding"),
            Self::InvalidEncryption => write!(f, "Invalid encryption (message integrity etc)"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidIdentification => write!(f, "Invalid identification"),
            Self::InvalidIdentity(e) => write!(f, "Invalid identity: {}", e),
            Self::NoCommonServerHostKeyAlgorithm => write!(f, "No common host key algorithm"),
            Self::NoCommonCompressionAlgorithm => write!(f, "No common compression algorithm"),
            Self::NoCommonEncryptionAlgorithm => write!(f, "No common encryption algorithm"),
            Self::NoCommonKexAlgorithm => write!(f, "No common kex algorithm"),
            Self::NoCommonMacAlgorithm => write!(f, "No common MAC algorithm"),
            Self::DisconnectByUs(r) => write!(f, "Disconnect by us: {}", r),
            Self::DisconnectByPeer(r) => write!(f, "Disconnect by peer: {}", r),
        }
    }
}
