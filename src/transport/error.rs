use super::*;

#[derive(Debug,Copy,Clone)]
pub enum TransportError {
    IoError(std::io::ErrorKind),
    DecoderError,
    KexError(KexError),
    BadPacketLength,
    MessageIntegrity,
    MessageUnimplemented(MsgUnimplemented),
    UnexpectedMessageType(u8),
    DisconnectByUs,
    DisconnectByPeer,
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
