use super::*;

pub type TransportResult<T> = Result<T,TransportError>;

#[derive(Debug)]
pub enum TransportError {
    IoError(std::io::Error),
    DecoderError,
    KexError(KexError),
    MessageIntegrity,
    UnexpectedMessageType(u8),
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<KexError> for TransportError {
    fn from(e: KexError) -> Self {
        Self::KexError(e)
    }
}
