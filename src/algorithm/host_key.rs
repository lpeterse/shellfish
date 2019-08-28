#[derive(Debug,Clone)]
pub enum HostKeyAlgorithm {
    SshEd25519,
    Unknown(Vec<u8>),
}

impl AsRef<[u8]> for HostKeyAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            HostKeyAlgorithm::SshEd25519 => b"ssh-ed25519",
            HostKeyAlgorithm::Unknown(s) => s,
        }
    }
}

impl From<&[u8]> for HostKeyAlgorithm {
    fn from(x: &[u8]) -> Self {
        if x == HostKeyAlgorithm::SshEd25519.as_ref() {
            HostKeyAlgorithm::SshEd25519
        } else {
            HostKeyAlgorithm::Unknown(Vec::from(x))
        }
    }
}