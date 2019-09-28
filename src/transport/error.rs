use super::*;

pub type TransportResult<T> = Result<T,TransportError>;

#[derive(Debug,Copy,Clone)]
pub enum TransportError {
    IoError(std::io::ErrorKind),
    DecoderError,
    KexError(KexError),
    BadPacketLength,
    MessageIntegrity,
    UnexpectedMessageType(u8),
    DisconnectByUs,
    DisconnectByPeer,
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
