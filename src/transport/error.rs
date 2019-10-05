use super::*;

use crate::transport::msg_disconnect::Reason;

#[derive(Debug,Copy,Clone)]
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
