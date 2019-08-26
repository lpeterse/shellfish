use num::BigUint;
use quickcheck::{Arbitrary, Gen};
use rand::Rng;

use crate::codec::*;
use crate::codec_ssh::*;

#[derive(PartialEq, Clone, Debug)]
pub struct RsaPublicKey {
    pub public_e: BigUint,
    pub public_n: BigUint,
}

impl <'a> SshCodec<'a> for RsaPublicKey {
    fn size(&self) -> usize {
        SshCodec::size(&self.public_e) +
        SshCodec::size(&self.public_n)
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        SshCodec::encode(&self.public_e, c);
        SshCodec::encode(&self.public_n, c);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let e = SshCodec::decode(c)?;
        let n = SshCodec::decode(c)?;
        Some(RsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}

impl Arbitrary for RsaPublicKey {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let e: usize = g.gen();
        let n: usize = g.gen();
        Self {
            public_e: BigUint::from(e),
            public_n: BigUint::from(n),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate quickcheck;
    extern crate quickcheck_macros;
    use quickcheck_macros::*;
    use super::*;

    #[quickcheck]
    fn test_ssh_codec_quick(x: RsaPublicKey) -> bool {
        let mut buf = vec![0;SshCodec::size(&x)];
        let mut encoder = Encoder::from(&mut buf[..]);
        SshCodec::encode(&x, &mut encoder);
        let mut decoder = Decoder::from(&buf[..]);
        SshCodec::decode(&mut decoder) == Some(x)
    }
}
