use crate::codec::*;
use crate::codec_ssh::*;

#[derive(Clone, Debug)]
pub struct Curve25519PublicKey ([u8;32]);

impl Curve25519PublicKey {
    pub fn new() -> Self {
        Self([7;32])
    }
}

impl <'a> SshCodec<'a> for Curve25519PublicKey {
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
        Some(Self(k))
    }
}
