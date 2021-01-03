mod files;

pub use self::files::*;

use crate::identity::*;
use crate::util::BoxFuture;

/// This trait captures the capability of verifying host identities.
///
/// The traditional mechanism for host-key verification is looking it up in system wide or user
/// specific `known_hosts` files. The struct `KnownHostsFiles` implements this mechanism.
pub trait KnownHostsLike: std::fmt::Debug + Send + Sync + 'static {
    /// Verify that the given hostname matches the presented identity.
    ///
    /// The result distiniguishes between errors to complete the verification process itself
    /// and its positive or negative outcome. `Ok(None)` shall be returned in case it is unknown
    /// whether the host shall be accepted or rejected. If no additional/alternative verification
    /// mechanism exists, this shall simply result in rejection as well.
    fn verify(&self, name: &str, identity: &Identity) -> BoxFuture<Result<(), KnownHostsError>>;
}

#[derive(Debug, Clone)]
pub enum KnownHostsError {
    Unverifiable,
    KeyRevoked,
    CertError(CertError),
    OtherError(String),
}

impl From<CertError> for KnownHostsError {
    fn from(e: CertError) -> Self {
        Self::CertError(e)
    }
}

impl From<std::io::Error> for KnownHostsError {
    fn from(e: std::io::Error) -> Self {
        Self::OtherError(format!("{}", e))
    }
}

impl std::fmt::Display for KnownHostsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unverifiable => write!(f, "Key/identity not found"),
            Self::KeyRevoked => write!(f, "Key has been revoked"),
            Self::CertError(e) => write!(f, "{}", e),
            Self::OtherError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for KnownHostsError {}
