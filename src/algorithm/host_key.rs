use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum HostKeyAlgorithm {
    SshEd25519,
    Unknown(String),
}

impl AsRef<[u8]> for HostKeyAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            HostKeyAlgorithm::SshEd25519 => b"ssh-ed25519",
            HostKeyAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for HostKeyAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(if x == HostKeyAlgorithm::SshEd25519.as_ref() {
            HostKeyAlgorithm::SshEd25519
        } else {
            HostKeyAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
        })
    }
}