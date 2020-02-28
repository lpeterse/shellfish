use crate::algorithm::authentication::*;

use std::future::Future;
use std::ops::Deref;

pub trait HostKeyVerifier: Send + Sync + 'static {
    fn verify(&self, identity: &HostIdentity) -> VerificationFuture;
}

impl HostKeyVerifier for Box<dyn HostKeyVerifier> {
    fn verify(&self, identity: &HostIdentity) -> VerificationFuture {
        self.deref().verify(identity)
    }
}

pub type VerificationFuture = Box<dyn Future<Output = Result<(), HostKeyVerificationError>> + Send + Unpin>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum HostKeyVerificationError {
    VerificationFailed
}

pub struct IgnorantVerifier {}

impl HostKeyVerifier for IgnorantVerifier {
    fn verify(&self, identity: &HostIdentity) -> VerificationFuture {
        log::warn!("DANGER: Blindly accepting host key {:?}", identity);
        Box::new(futures::future::ready(Ok(())))
    }
}
