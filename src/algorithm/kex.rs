use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum KexAlgorithm {
    Curve25519Sha256,
    Curve25519Sha256AtLibsshDotOrg,
    EcdhSha2Nistp256,
    EcdhSha2Nistp384,
    EcdhSha2Nistp521,
    DiffieHellmanGroupExchangeSha256,
    DiffieHellmanGroup16Sha512,
    DiffieHellmanGroup8Sha512,
    DiffieHellmanGroup14Sha256,
    DiffieHellmanGroup14Sha1,
    Unknown(String),
}

impl AsRef<[u8]> for KexAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            KexAlgorithm::Curve25519Sha256 => b"curve25519-sha256",
            KexAlgorithm::Curve25519Sha256AtLibsshDotOrg => b"curve25519-sha256@libssh.org",
            KexAlgorithm::EcdhSha2Nistp256 => b"ecdh-sha2-nistp256",
            KexAlgorithm::EcdhSha2Nistp384 => b"ecdh-sha2-nistp384",
            KexAlgorithm::EcdhSha2Nistp521 => b"ecdh-sha2-nistp521",
            KexAlgorithm::DiffieHellmanGroupExchangeSha256 => b"diffie-hellman-group-exchange-sha256",
            KexAlgorithm::DiffieHellmanGroup16Sha512 => b"diffie-hellman-group16-sha512",
            KexAlgorithm::DiffieHellmanGroup8Sha512 => b"diffie-hellman-group18-sha512",
            KexAlgorithm::DiffieHellmanGroup14Sha256 => b"diffie-hellman-group14-sha256",
            KexAlgorithm::DiffieHellmanGroup14Sha1 => b"diffie-hellman-group14-sha1",
            KexAlgorithm::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for KexAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(
            if x == KexAlgorithm::Curve25519Sha256AtLibsshDotOrg.as_ref() {
                KexAlgorithm::Curve25519Sha256AtLibsshDotOrg
            }
            else if x == KexAlgorithm::Curve25519Sha256.as_ref() {
                KexAlgorithm::Curve25519Sha256
            }
            else if x == KexAlgorithm::EcdhSha2Nistp256.as_ref() {
                KexAlgorithm::EcdhSha2Nistp256
            }
            else if x == KexAlgorithm::EcdhSha2Nistp384.as_ref() {
                KexAlgorithm::EcdhSha2Nistp384
            }
            else if x == KexAlgorithm::EcdhSha2Nistp521.as_ref() {
                KexAlgorithm::EcdhSha2Nistp521
            }
            else if x == KexAlgorithm::DiffieHellmanGroupExchangeSha256.as_ref() {
                KexAlgorithm::DiffieHellmanGroupExchangeSha256
            }
            else if x == KexAlgorithm::DiffieHellmanGroup16Sha512.as_ref() {
                KexAlgorithm::DiffieHellmanGroup16Sha512
            }
            else if x == KexAlgorithm::DiffieHellmanGroup8Sha512.as_ref() {
                KexAlgorithm::DiffieHellmanGroup8Sha512
            }
            else if x == KexAlgorithm::DiffieHellmanGroup14Sha256.as_ref() {
                KexAlgorithm::DiffieHellmanGroup14Sha256
            }
            else if x == KexAlgorithm::DiffieHellmanGroup14Sha1.as_ref() {
                KexAlgorithm::DiffieHellmanGroup14Sha1
            }
            else {
                KexAlgorithm::Unknown(String::from_utf8(Vec::from(x))?)
            }
        )
    }
}
