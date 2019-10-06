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

impl EncryptionAlgorithm {
    pub fn supported() -> Vec<Self> {
        vec![Self::Chacha20Poly1305AtOpensshDotCom]
    }
}

impl AsRef<[u8]> for EncryptionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Chacha20Poly1305AtOpensshDotCom => b"chacha20-poly1305@openssh.com",
            Self::Aes128Ctr => b"aes128-ctr",
            Self::Aes192Ctr => b"aes192-ctr",
            Self::Aes256Ctr => b"aes256-ctr",
            Self::Aes128GcmAtOpensshDotcom => b"aes128-gcm@openssh.com",
            Self::Aes256GcmAtOpensshDotcom => b"aes256-gcm@openssh.com",
            Self::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for EncryptionAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(
            if x == Self::Chacha20Poly1305AtOpensshDotCom.as_ref() {
                Self::Chacha20Poly1305AtOpensshDotCom
            }
            else if x == Self::Aes128Ctr.as_ref() {
                Self::Aes128Ctr
            }
            else if x == Self::Aes192Ctr.as_ref() {
                Self::Aes192Ctr
            }
            else if x == Self::Aes256Ctr.as_ref() {
                Self::Aes256Ctr
            }
            else if x == Self::Aes128GcmAtOpensshDotcom.as_ref() {
                Self::Aes128GcmAtOpensshDotcom
            }
            else if x == Self::Aes256GcmAtOpensshDotcom.as_ref() {
                Self::Aes256GcmAtOpensshDotcom
            }
            else {
                Self::Unknown(String::from_utf8(Vec::from(x))?)
            }
        )
    }
}
