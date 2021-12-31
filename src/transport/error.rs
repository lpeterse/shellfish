use super::*;
use crate::identity::SignatureError;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum TransportError {
    IoError(Arc<std::io::Error>),
    AgentError(AuthAgentError),
    AgentRefusedToSign,
    InvalidEncoding,
    InvalidState,
    InvalidPacket,
    InvalidPacketLength,
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

impl From<SshCodecError> for TransportError {
    fn from(_: SshCodecError) -> Self {
        Self::InvalidEncoding
    }
}

impl From<AuthAgentError> for TransportError {
    fn from(e: AuthAgentError) -> Self {
        Self::AgentError(e)
    }
}

impl From<SignatureError> for TransportError {
    fn from(_: SignatureError) -> Self {
        Self::InvalidSignature
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "{}", e),
            Self::AgentError(e) => write!(f, "Agent error: {}", e),
            Self::AgentRefusedToSign => write!(f, "Agent refused to sign"),
            Self::InvalidEncoding => write!(f, "Invalid encoding"),
            Self::InvalidState => write!(f, "Invalid state (protocol error)"),
            Self::InvalidPacket => write!(f, "Invalid packet structure"),
            Self::InvalidPacketLength => write!(f, "Invalid packet length"),
            Self::InvalidEncryption => write!(f, "Invalid encryption (message integrity etc)"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidIdentification => write!(f, "Invalid identification"),
            Self::InvalidIdentity(e) => write!(f, "Invalid identity: {}", e),
            Self::NoCommonServerHostKeyAlgorithm => write!(f, "No common server host key algorithm"),
            Self::NoCommonCompressionAlgorithm => write!(f, "No common compression algorithm"),
            Self::NoCommonEncryptionAlgorithm => write!(f, "No common encryption algorithm"),
            Self::NoCommonKexAlgorithm => write!(f, "No common kex algorithm"),
            Self::NoCommonMacAlgorithm => write!(f, "No common MAC algorithm"),
            Self::DisconnectByUs(r) => write!(f, "Disconnect by us: {}", r),
            Self::DisconnectByPeer(r) => write!(f, "Disconnect by peer: {}", r),
        }
    }
}
