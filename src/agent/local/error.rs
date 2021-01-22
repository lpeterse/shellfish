#[derive(Debug)]
pub enum LocalAgentError {
    FrameLengthError,
    EncodingError,
}

impl std::fmt::Display for LocalAgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameLengthError => write!(f, "invalid frame length"),
            Self::EncodingError => write!(f, "encoding/decoding failed"),
        }
    }
}

impl std::error::Error for LocalAgentError {}
