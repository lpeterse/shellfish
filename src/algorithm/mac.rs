use std::convert::TryFrom;

#[derive(Debug,Clone)]
pub enum MacAlgorithm {
    Unknown(String),
}

impl AsRef<[u8]> for MacAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            MacAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for MacAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(MacAlgorithm::Unknown(String::from_utf8(Vec::from(x))?))
    }
}
