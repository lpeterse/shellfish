use crate::algorithm::auth::*;

use async_std::future::ready;
use std::future::Future;
use std::ops::Deref;

pub trait HostKeyVerifier: std::fmt::Debug + Send + Sync + 'static {
    fn verify(&self, name: &str, identity: &Identity) -> VerificationFuture;
}

impl HostKeyVerifier for Box<dyn HostKeyVerifier> {
    fn verify(&self, name: &str, identity: &Identity) -> VerificationFuture {
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

#[derive(Debug)]
pub struct AcceptingVerifier {}

impl HostKeyVerifier for AcceptingVerifier {
    fn verify(&self, name: &str, identity: &Identity) -> VerificationFuture {
        log::warn!(
            "DANGER: Blindly accepting host key {:?} for {}",
            identity,
            name
        );
        Box::pin(ready(Ok(())))
    }
}
