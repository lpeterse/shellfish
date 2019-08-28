#[derive(Debug,Clone)]
pub enum KexAlgorithm {
    Curve25519Sha256AtLibsshDotOrg,
    Unknown(Vec<u8>),
}

impl AsRef<[u8]> for KexAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            KexAlgorithm::Curve25519Sha256AtLibsshDotOrg => b"curve25519-sha256@libssh.org",
            KexAlgorithm::Unknown(s) => s,
        }
    }
}

impl From<&[u8]> for KexAlgorithm {
    fn from(x: &[u8]) -> Self {
        if x == KexAlgorithm::Curve25519Sha256AtLibsshDotOrg.as_ref() {
            KexAlgorithm::Curve25519Sha256AtLibsshDotOrg
        } else {
            KexAlgorithm::Unknown(Vec::from(x))
        }
    }
}
