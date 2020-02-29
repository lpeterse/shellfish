use crate::algorithm::authentication::*;

use std::future::Future;
use std::ops::Deref;

pub trait HostKeyVerifier: Send + Sync + 'static {
    fn verify(&self, hostname: &str, identity: &HostIdentity) -> VerificationFuture;
}

impl HostKeyVerifier for Box<dyn HostKeyVerifier> {
    fn verify(&self, hostname: &str, identity: &HostIdentity) -> VerificationFuture {
        self.deref().verify(hostname, identity)
    }
}

pub type VerificationFuture =
    Box<dyn Future<Output = Result<(), HostKeyVerificationError>> + Send + Unpin>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum HostKeyVerificationError {
    VerificationFailed,
}

pub struct AcceptingVerifier {}

impl HostKeyVerifier for AcceptingVerifier {
    fn verify(&self, hostname: &str, identity: &HostIdentity) -> VerificationFuture {
        log::warn!("DANGER: Blindly accepting host key {:?} for {}", identity, hostname);
        Box::new(futures::future::ready(Ok(())))
    }
}

pub struct RejectingVerifier {}

impl HostKeyVerifier for RejectingVerifier {
    fn verify(&self, hostname: &str, identity: &HostIdentity) -> VerificationFuture {
        log::error!("DANGER: Rejecting host key {:?} for {}", identity, hostname);
        Box::new(futures::future::ready(Err(
            HostKeyVerificationError::VerificationFailed,
        )))
    }
}
