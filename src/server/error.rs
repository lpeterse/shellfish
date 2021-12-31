use crate::transport::TransportError;
use crate::user_auth::UserAuthError;

use std::error::Error;

#[derive(Debug)]
pub enum ServerError {
    SocketError(std::io::Error),
    TransportError(TransportError),
    UserAuthError(UserAuthError),
}

impl Error for ServerError {}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<TransportError> for ServerError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<UserAuthError> for ServerError {
    fn from(e: UserAuthError) -> Self {
        Self::UserAuthError(e)
    }
}
