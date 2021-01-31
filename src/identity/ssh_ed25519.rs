use crate::util::codec::*;
use std::convert::TryInto;

#[derive(Debug)]
pub struct SshEd25519;

impl SshEd25519 {
    pub const NAME: &'static str = "ssh-ed25519";
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshEd25519PublicKey<'a>(pub &'a [u8; 32]);

impl<'a> SshEd25519PublicKey<'a> {
    pub fn pk(&self) -> &[u8; 32] {
        self.0
    }
}

impl<'a> SshEncode for SshEd25519PublicKey<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(SshEd25519::NAME)?;
        e.push_bytes_framed(self.0)
    }
}

impl<'a> SshDecodeRef<'a> for SshEd25519PublicKey<'a> {
    fn decode<D: SshDecoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_str_framed(SshEd25519::NAME)?;
        c.take_bytes_framed()?.try_into().ok().map(Self)
    }
}

#[cfg(test)]
mod tests {}
