use num::BigUint;

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
