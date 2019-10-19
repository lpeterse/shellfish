use crate::algorithm::authentication::*;

use std::ops::Deref;

pub trait HostKeyVerifier: Send + Sync + 'static {
    fn verify(&self, identity: &HostIdentity) -> bool;
}

impl HostKeyVerifier for Box<dyn HostKeyVerifier> {
    fn verify(&self, identity: &HostIdentity) -> bool {
        self.deref().verify(identity)
    }
}

pub struct IgnorantVerifier {}

impl HostKeyVerifier for IgnorantVerifier {
    fn verify(&self, identity: &HostIdentity) -> bool {
        log::warn!("DANGER: Blindly accepting host key {:?}", identity);
        true
    }
}
