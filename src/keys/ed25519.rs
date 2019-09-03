use crate::codec::*;
use crate::codec_ssh::*;

#[derive(PartialEq, Clone, Debug)]
pub struct Ed25519PublicKey ([u8;32]);

impl Ed25519PublicKey {
    pub fn new() -> Self {
        Self([7;32])
    }
}

impl <'a> SshCodec<'a> for Ed25519PublicKey {
    fn size(&self) -> usize {
        4 + 32
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u32be(32);
        c.push_bytes(&self.0);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let mut k = [0;32];
        c.take_u32be().filter(|x| x == &32)?;
        c.take_bytes_into(&mut k)?;
        Some(Ed25519PublicKey(k))
    }
}


pub struct Ed25519Signature ([u8;64]);

impl Clone for Ed25519Signature {
    fn clone(&self) -> Self {
        Self (self.0)
    }
}

impl std::fmt::Debug for Ed25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature {:?}..", &self.0[..32])
    }
}

impl <'a> SshCodec<'a> for Ed25519Signature {
    fn size(&self) -> usize {
        4 + 64
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u32be(64);
        c.push_bytes(&self.0);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let mut k = [0;64];
        c.take_u32be().filter(|x| x == &64)?;
        c.take_bytes_into(&mut k)?;
        Some(Self(k))
    }
}
