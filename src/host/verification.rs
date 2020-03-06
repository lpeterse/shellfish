use crate::algorithm::authentication::*;

use futures::future::FutureExt;
use std::future::Future;
use std::ops::Deref;

pub trait HostKeyVerifier: Send + Sync + 'static {
    fn verify(&self, name: &str, identity: &HostIdentity) -> VerificationFuture;
}

impl HostKeyVerifier for Box<dyn HostKeyVerifier> {
    fn verify(&self, name: &str, identity: &HostIdentity) -> VerificationFuture {
        self.deref().verify(name, identity)
    }
}

pub type VerificationFuture =
    core::pin::Pin<Box<dyn Future<Output = Result<(), VerificationError>> + Send>>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VerificationError {
    FileError(std::io::ErrorKind),
    KeyRevoked,
    KeyNotFound,
}

pub struct AcceptingVerifier {}

impl HostKeyVerifier for AcceptingVerifier {
    fn verify(&self, name: &str, identity: &HostIdentity) -> VerificationFuture {
        log::warn!(
            "DANGER: Blindly accepting host key {:?} for {}",
            identity,
            name
        );
        futures::future::ready(Ok(())).boxed()
    }
}

pub struct RejectingVerifier {}

impl HostKeyVerifier for RejectingVerifier {
    fn verify(&self, name: &str, identity: &HostIdentity) -> VerificationFuture {
        log::error!("DANGER: Rejecting host key {:?} for {}", identity, name);
        futures::future::ready(Err(VerificationError::KeyNotFound)).boxed()
    }
}
