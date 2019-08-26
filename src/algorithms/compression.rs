#[derive(Debug,Clone)]
pub enum CompressionAlgorithm {
    None,
    Unknown(Vec<u8>)
}

impl AsRef<[u8]> for CompressionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            CompressionAlgorithm::None => b"none",
            CompressionAlgorithm::Unknown(s) => s,
        }
    }
}

impl From<&[u8]> for CompressionAlgorithm {
    fn from(x: &[u8]) -> Self {
        if x == CompressionAlgorithm::None.as_ref() {
            CompressionAlgorithm::None
        } else {
            CompressionAlgorithm::Unknown(Vec::from(x))
        }
    }
}