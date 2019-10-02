use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Curve25519PublicKey ([u8;32]);

impl Curve25519PublicKey {
    const SIZE: u32 = 32;
}

impl Encode for Curve25519PublicKey {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + Self::SIZE as usize
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(Self::SIZE);
        c.push_bytes(&self.0);
    }
}

impl Decode for Curve25519PublicKey {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let mut k = [0;32];
        c.expect_u32be(Self::SIZE)?;
        c.take_into(&mut k)?;
        Some(Self(k))
    }
}
