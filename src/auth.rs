mod agent;
mod certificate;
mod identity;
mod signature;
mod user_auth;

pub mod ssh_ed25519;
pub mod ssh_ed25519_cert;
pub mod ssh_rsa;

pub use self::agent::*;
pub use self::certificate::*;
pub use self::identity::*;
pub use self::signature::*;
pub use self::user_auth::*;

use crate::util::codec::*;

pub(crate) const HOST_KEY_ALGORITHMS: [&'static str; 1] = [ssh_ed25519::SshEd25519::NAME];
