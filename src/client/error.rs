use super::*;

#[derive(Debug)]
pub enum ClientError {
    ConnectError(std::io::Error),
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
