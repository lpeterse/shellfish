#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SshCodecError {
    EncodingFailed,
    DecodingFailed
}

impl std::error::Error for SshCodecError {}

impl std::fmt::Display for SshCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
