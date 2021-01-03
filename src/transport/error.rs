use super::*;

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
    InvalidHostKey(KnownHostsError),
    NoCommonServerHostKeyAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorithm,
    DisconnectByUs(DisconnectReason),
    DisconnectByPeer(DisconnectReason),
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(Arc::new(e))
    }
}
