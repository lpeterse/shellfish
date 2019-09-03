use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum KexAlgorithm {
    Curve25519Sha256AtLibsshDotOrg,
    Unknown(String),
}

impl AsRef<[u8]> for KexAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            KexAlgorithm::Curve25519Sha256AtLibsshDotOrg => b"curve25519-sha256@libssh.org",
            KexAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for KexAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(if x == KexAlgorithm::Curve25519Sha256AtLibsshDotOrg.as_ref() {
            KexAlgorithm::Curve25519Sha256AtLibsshDotOrg
        } else {
            KexAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
        })
    }
}
