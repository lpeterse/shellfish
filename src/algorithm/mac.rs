use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum MacAlgorithm {
    Umac64EtmAtOpensshDotCom,
    Umac128EtmAtOpensshDotCom,
    HmacSha2_256EtmAtOpensshDotCom,
    HmacSha2_512EtmAtOpensshDotCom,
    HmacSha1EtmAtOpensshDotCom,
    Umac64AtOpensshDotCom,
    Umac128AtOpensshDotCom,
    HmacSha2_256,
    HmacSha2_512,
    HmacSha1,
    Unknown(String),
}

impl MacAlgorithm {
    pub fn supported() -> Vec<Self> {
        vec![]
    }
}

impl AsRef<[u8]> for MacAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Umac64EtmAtOpensshDotCom => b"umac-64-etm@openssh.com",
            Self::Umac128EtmAtOpensshDotCom => b"umac-128-etm@openssh.com",
            Self::HmacSha2_256EtmAtOpensshDotCom => b"hmac-sha2-256-etm@openssh.com",
            Self::HmacSha2_512EtmAtOpensshDotCom => b"hmac-sha2-512-etm@openssh.com",
            Self::HmacSha1EtmAtOpensshDotCom => b"hmac-sha1-etm@openssh.com",
            Self::Umac64AtOpensshDotCom => b"umac-64@openssh.com",
            Self::Umac128AtOpensshDotCom => b"umac-128@openssh.com",
            Self::HmacSha2_256 => b"hmac-sha2-256",
            Self::HmacSha2_512 => b"hmac-sha2-512",
            Self::HmacSha1 => b"hmac-sha1",
            Self::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for MacAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(
            if x == Self::Umac64EtmAtOpensshDotCom.as_ref() {
                Self::Umac64EtmAtOpensshDotCom
            }
            else if x == Self::Umac128EtmAtOpensshDotCom.as_ref() {
                Self::Umac128EtmAtOpensshDotCom
            }
            else if x == Self::HmacSha2_256EtmAtOpensshDotCom.as_ref() {
                Self::HmacSha2_256EtmAtOpensshDotCom
            }
            else if x == Self::HmacSha2_512EtmAtOpensshDotCom.as_ref() {
                Self::HmacSha2_512EtmAtOpensshDotCom
            }
            else if x == Self::HmacSha1EtmAtOpensshDotCom.as_ref() {
                Self::HmacSha1EtmAtOpensshDotCom
            }
            else if x == Self::Umac64AtOpensshDotCom.as_ref() {
                Self::Umac64AtOpensshDotCom
            }
            else if x == Self::Umac128AtOpensshDotCom.as_ref() {
                Self::Umac128AtOpensshDotCom
            }
            else if x == Self::HmacSha2_256.as_ref() {
                Self::HmacSha2_256
            }
            else if x == Self::HmacSha2_512.as_ref() {
                Self::HmacSha2_512
            }
            else if x == Self::HmacSha1.as_ref() {
                Self::HmacSha1
            }
            else {
                Self::Unknown(String::from_utf8(Vec::from(x))?)
            }
        )
    }
}
