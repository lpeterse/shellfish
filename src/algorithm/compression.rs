use std::convert::TryFrom;

#[derive(Debug,Clone)]
pub enum CompressionAlgorithm {
    None,
    Unknown(String)
}

impl AsRef<[u8]> for CompressionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            CompressionAlgorithm::None => b"none",
            CompressionAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for CompressionAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(if x == CompressionAlgorithm::None.as_ref() {
            CompressionAlgorithm::None
        } else {
            CompressionAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
        })
    }
}