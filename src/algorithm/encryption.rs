use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum EncryptionAlgorithm {
    Chacha20Poly1305AtOpensshDotCom,
    Aes128Ctr,
    Aes192Ctr,
    Aes256Ctr,
    Aes128GcmAtOpensshDotcom,
    Aes256GcmAtOpensshDotcom,
    Unknown(String)
}

impl AsRef<[u8]> for EncryptionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom => b"chacha20-poly1305@openssh.com",
            EncryptionAlgorithm::Aes128Ctr => b"aes128-ctr",
            EncryptionAlgorithm::Aes192Ctr => b"aes192-ctr",
            EncryptionAlgorithm::Aes256Ctr => b"aes256-ctr",
            EncryptionAlgorithm::Aes128GcmAtOpensshDotcom => b"aes128-gcm@openssh.com",
            EncryptionAlgorithm::Aes256GcmAtOpensshDotcom => b"aes256-gcm@openssh.com",
            EncryptionAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for EncryptionAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(
            if x == EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom.as_ref() {
                EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom
            }
            else if x == EncryptionAlgorithm::Aes128Ctr.as_ref() {
                EncryptionAlgorithm::Aes128Ctr
            }
            else if x == EncryptionAlgorithm::Aes192Ctr.as_ref() {
                EncryptionAlgorithm::Aes192Ctr
            }
            else if x == EncryptionAlgorithm::Aes256Ctr.as_ref() {
                EncryptionAlgorithm::Aes256Ctr
            }
            else if x == EncryptionAlgorithm::Aes128GcmAtOpensshDotcom.as_ref() {
                EncryptionAlgorithm::Aes128GcmAtOpensshDotcom
            }
            else if x == EncryptionAlgorithm::Aes256GcmAtOpensshDotcom.as_ref() {
                EncryptionAlgorithm::Aes256GcmAtOpensshDotcom
            }
            else {
                EncryptionAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
            }
        )
    }
}
