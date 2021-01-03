mod cert;
mod identity;
mod signature;

pub mod ssh_ed25519;
pub mod ssh_ed25519_cert;
pub mod ssh_rsa;

pub use self::cert::*;
pub use self::identity::*;
pub use self::signature::*;

pub(crate) const HOST_KEY_ALGORITHMS: [&'static str; 1] = [ssh_ed25519::SshEd25519::NAME];
