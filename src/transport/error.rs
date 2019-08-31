pub type TransportResult<T> = Result<T,TransportError>;

#[derive(Debug)]
pub enum TransportError {
    IoError(std::io::Error),
    InvalidIdentification,
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
