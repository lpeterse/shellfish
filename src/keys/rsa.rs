use num::BigUint;

use crate::codec::*;

#[derive(PartialEq, Clone, Debug)]
pub struct RsaPublicKey {
    pub public_e: BigUint,
    pub public_n: BigUint,
}

impl <'a> Codec<'a> for RsaPublicKey {
    fn size(&self) -> usize {
        Codec::size(&self.public_e) +
        Codec::size(&self.public_n)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        Codec::encode(&self.public_e, c);
        Codec::encode(&self.public_n, c);
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let e = Codec::decode(c)?;
        let n = Codec::decode(c)?;
        Some(RsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}
