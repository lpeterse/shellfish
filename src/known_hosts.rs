mod files;

pub use self::files::*;

use crate::auth::*;
use crate::util::BoxFuture;

use async_std::future::ready;
use std::ops::Deref;

pub type KnownHostsError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type KnownHostsResult = Result<Option<KnownHostsDecision>, KnownHostsError>;
pub type KnownHostsFuture = BoxFuture<KnownHostsResult>;

pub enum KnownHostsDecision {
    Accepted,
    Rejected,
}

pub trait KnownHosts: std::fmt::Debug + Send + Sync + 'static {
    fn verify(&self, name: &str, identity: &Identity) -> KnownHostsFuture;
}

impl KnownHosts for Box<dyn KnownHosts> {
    fn verify(&self, name: &str, identity: &Identity) -> KnownHostsFuture {
        self.deref().verify(name, identity)
    }
}

#[derive(Debug)]
pub struct AcceptingVerifier {}

impl KnownHosts for AcceptingVerifier {
    fn verify(&self, name: &str, identity: &Identity) -> KnownHostsFuture {
        log::warn!(
            "DANGER: Blindly accepting host key {:?} for {}",
            identity,
            name
        );
        Box::pin(ready(Ok(Some(KnownHostsDecision::Accepted))))
    }
}
