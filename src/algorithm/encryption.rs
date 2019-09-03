use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum EncryptionAlgorithm {
    Chacha20Poly1305AtOpensshDotCom,
    Unknown(String)
}

impl AsRef<[u8]> for EncryptionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom => b"chacha20-poly1305@openssh.com",
            EncryptionAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for EncryptionAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(if x == EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom.as_ref() {
            EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom
        } else {
            EncryptionAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
        })
    }
}