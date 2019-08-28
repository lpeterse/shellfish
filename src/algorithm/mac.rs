#[derive(Debug,Clone)]
pub enum MacAlgorithm {
    Unknown(Vec<u8>),
}

impl AsRef<[u8]> for MacAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            MacAlgorithm::Unknown(s) => s,
        }
    }
}

impl From<&[u8]> for MacAlgorithm {
    fn from(x: &[u8]) -> Self {
        MacAlgorithm::Unknown(Vec::from(x))
    }
}