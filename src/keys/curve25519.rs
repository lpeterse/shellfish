use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Curve25519PublicKey ([u8;32]);

impl Curve25519PublicKey {
    pub fn new() -> Self {
        Self([7;32])
    }
}

impl <'a> Codec<'a> for Curve25519PublicKey {
    fn size(&self) -> usize {
        4 + 32
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(32);
        c.push_bytes(&self.0);
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let mut k = [0;32];
        c.take_u32be().filter(|x| x == &32)?;
        c.take_into(&mut k)?;
        Some(Self(k))
    }
}
