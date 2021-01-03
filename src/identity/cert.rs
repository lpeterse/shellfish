mod error;
mod extension;
mod option;
mod type_;

pub use self::error::*;
pub use self::extension::*;
pub use self::option::*;
pub use self::type_::*;

use super::identity::*;
use std::net::IpAddr;

/// This trait shall be implemented for types that are certificates
/// (either ssh-cert or X.509 certificates).
pub trait Cert: Sync + Send + 'static {
    fn authority(&self) -> &Identity;

    fn verify_for_host(&self, hostname: &str) -> Result<(), CertError>;
    fn verify_for_client(&self, username: &str, source: &IpAddr) -> Result<(), CertError>;
}
