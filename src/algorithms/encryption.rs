#[derive(Debug,Clone)]
pub enum EncryptionAlgorithm {
    Chacha20Poly1305AtOpensshDotCom,
    Unknown(Vec<u8>)
}

impl AsRef<[u8]> for EncryptionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom => b"chacha20-poly1305@openssh.com",
            EncryptionAlgorithm::Unknown(s) => s,
        }
    }
}

impl From<&[u8]> for EncryptionAlgorithm {
    fn from(x: &[u8]) -> Self {
        if x == EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom.as_ref() {
            EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom
        } else {
            EncryptionAlgorithm::Unknown(Vec::from(x))
        }
    }
}