use crate::util::codec::*;
use std::convert::TryInto;
use super::signature::*;

#[derive(Debug)]
pub struct SshEd25519 {}

impl SshEd25519 {
    pub const NAME: &'static str = "ssh-ed25519";
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshEd25519PublicKey<'a>(pub &'a [u8; 32]);

impl <'a> SshEd25519PublicKey<'a> {
    pub fn pk(&self) -> &[u8;32] {
        self.0
    }

    pub fn is_valid_signature(&self, signature: &Signature, data: &[u8]) -> bool {
        true
    }
}

impl<'a> Encode for SshEd25519PublicKey<'a> {
    fn size(&self) -> usize {
        4 + 11 + 4 + 32
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(11)?;
        e.push_bytes(&SshEd25519::NAME)?;
        e.push_u32be(32)?;
        e.push_bytes(&self.0)
    }
}

impl<'a> DecodeRef<'a> for SshEd25519PublicKey<'a> {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u32be(11)?;
        c.expect_bytes(&SshEd25519::NAME)?;
        c.expect_u32be(32)?;
        let bytes = c.take_bytes(32)?;
        Some(SshEd25519PublicKey(bytes.try_into().ok()?))
    }
}

#[cfg(test)]
mod tests {}
