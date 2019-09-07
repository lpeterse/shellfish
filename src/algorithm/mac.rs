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

impl AsRef<[u8]> for MacAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            MacAlgorithm::Umac64EtmAtOpensshDotCom => b"umac-64-etm@openssh.com",
            MacAlgorithm::Umac128EtmAtOpensshDotCom => b"umac-128-etm@openssh.com",
            MacAlgorithm::HmacSha2_256EtmAtOpensshDotCom => b"hmac-sha2-256-etm@openssh.com",
            MacAlgorithm::HmacSha2_512EtmAtOpensshDotCom => b"hmac-sha2-512-etm@openssh.com",
            MacAlgorithm::HmacSha1EtmAtOpensshDotCom => b"hmac-sha1-etm@openssh.com",
            MacAlgorithm::Umac64AtOpensshDotCom => b"umac-64@openssh.com",
            MacAlgorithm::Umac128AtOpensshDotCom => b"umac-128@openssh.com",
            MacAlgorithm::HmacSha2_256 => b"hmac-sha2-256",
            MacAlgorithm::HmacSha2_512 => b"hmac-sha2-512",
            MacAlgorithm::HmacSha1 => b"hmac-sha1",
            MacAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for MacAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(
            if x == MacAlgorithm::Umac64EtmAtOpensshDotCom.as_ref() {
                MacAlgorithm::Umac64EtmAtOpensshDotCom
            }
            else if x == MacAlgorithm::Umac128EtmAtOpensshDotCom.as_ref() {
                MacAlgorithm::Umac128EtmAtOpensshDotCom
            }
            else if x == MacAlgorithm::HmacSha2_256EtmAtOpensshDotCom.as_ref() {
                MacAlgorithm::HmacSha2_256EtmAtOpensshDotCom
            }
            else if x == MacAlgorithm::HmacSha2_512EtmAtOpensshDotCom.as_ref() {
                MacAlgorithm::HmacSha2_512EtmAtOpensshDotCom
            }
            else if x == MacAlgorithm::HmacSha1EtmAtOpensshDotCom.as_ref() {
                MacAlgorithm::HmacSha1EtmAtOpensshDotCom
            }
            else if x == MacAlgorithm::Umac64AtOpensshDotCom.as_ref() {
                MacAlgorithm::Umac64AtOpensshDotCom
            }
            else if x == MacAlgorithm::Umac128AtOpensshDotCom.as_ref() {
                MacAlgorithm::Umac128AtOpensshDotCom
            }
            else if x == MacAlgorithm::HmacSha2_256.as_ref() {
                MacAlgorithm::HmacSha2_256
            }
            else if x == MacAlgorithm::HmacSha2_512.as_ref() {
                MacAlgorithm::HmacSha2_512
            }
            else if x == MacAlgorithm::HmacSha1.as_ref() {
                MacAlgorithm::HmacSha1
            }
            else {
                MacAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
            }
        )
    }
}
