use super::*;

use std::error::Error;

/// The client error is a supertype for all that might occur when working with a client.
#[derive(Clone, Debug)]
pub enum ClientError {
    TransportError(TransportError),
    UserAuthError(UserAuthError),
    ConnectionError(ConnectionError),
}

impl Error for ClientError {}

impl From<UserAuthError> for ClientError {
    fn from(e: UserAuthError) -> Self {
        Self::UserAuthError(e)
    }
}

impl From<TransportError> for ClientError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<ConnectionError> for ClientError {
    fn from(e: ConnectionError) -> Self {
        Self::ConnectionError(e)
    }
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransportError(e) => write!(f, "Transport: {}", e),
            Self::UserAuthError(e) => write!(f, "UserAuth: {}", e),
            Self::ConnectionError(e) => write!(f, "Connection: {}", e),
        }
    }
}
