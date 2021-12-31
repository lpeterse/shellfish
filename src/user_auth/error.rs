use crate::agent::AuthAgentError;
use crate::transport::TransportError;
use crate::util::codec::SshCodecError;
use std::error::Error;

#[derive(Clone, Debug)]
pub enum UserAuthError {
    CodecError(SshCodecError),
    AuthAgentError(AuthAgentError),
    TransportError(TransportError),
    NoMoreAuthMethods,
}

impl From<SshCodecError> for UserAuthError {
    fn from(e: SshCodecError) -> Self {
        Self::CodecError(e)
    }
}

impl From<AuthAgentError> for UserAuthError {
    fn from(e: AuthAgentError) -> Self {
        Self::AuthAgentError(e)
    }
}

impl From<TransportError> for UserAuthError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl Error for UserAuthError {}

impl std::fmt::Display for UserAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CodecError(e) => write!(f, "Codec error: {}", e),
            Self::AuthAgentError(e) => write!(f, "Auth agent: {}", e),
            Self::TransportError(e) => write!(f, "Transport: {}", e),
            Self::NoMoreAuthMethods => write!(f, "No more auth methods"),
        }
    }
}
