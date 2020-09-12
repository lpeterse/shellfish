use super::*;

use std::error::Error;

#[derive(Debug)]
pub enum ClientError {
    ConnectError(std::io::ErrorKind),
    TransportError(TransportError),
    UserAuthError(UserAuthError),
    ConnectionError(ConnectionError),
}

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

impl Error for ClientError {}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
