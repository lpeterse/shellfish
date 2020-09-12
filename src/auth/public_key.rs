use super::ssh_ed25519::*;
use super::ssh_rsa::*;
use super::*;

use std::any::Any;
use std::ops::Deref;

pub trait PublicKey {
    fn algorithm(&self) -> &str;
    fn verify_signature(&self, signature: &Signature, data: &[u8]) -> bool;
    fn equals(&self, public_key: &Box<dyn PublicKey>) -> bool;
    fn as_any(&self) -> &dyn Any;
}

impl<T: PublicKey> PublicKey for Box<T> {
    fn algorithm(&self) -> &str {
        self.deref().algorithm()
    }
    fn verify_signature(&self, signature: &Signature, data: &[u8]) -> bool {
        self.deref().verify_signature(signature, data)
    }
    fn equals(&self, public_key: &Box<dyn PublicKey>) -> bool {
        self.deref().equals(public_key)
    }
    fn as_any(&self) -> &dyn Any {
        self.deref().as_any()
    }
}

pub fn decode_public_key(input: &[u8]) -> Option<Box<dyn PublicKey>> {
    if let Some(pk) = SliceDecoder::decode(input) {
        let _: Ed25519PublicKey = pk;
        return Some(Box::new(pk));
    }
    if let Some(pk) = SliceDecoder::decode(input) {
        let _: RsaPublicKey = pk;
        return Some(Box::new(pk));
    }
    None
}
