use super::*;
use crate::codec::*;
use num::BigUint;

pub struct SshRsa {}

impl SignatureAlgorithm for SshRsa {
    type PublicKey = SshRsaPublicKey;
    type PrivateKey = ();
    type Signature = ();

    const NAME: &'static str = "ssh-rsa";
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshRsaPublicKey {
    pub public_e: BigUint,
    pub public_n: BigUint,
}

impl Encode for SshRsaPublicKey {
    fn size(&self) -> usize {
        4 + Encode::size(&<SshRsa as SignatureAlgorithm>::NAME)
            + Encode::size(&self.public_e)
            + Encode::size(&self.public_n)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be((Encode::size(self) - 4) as u32);
        Encode::encode(&<SshRsa as SignatureAlgorithm>::NAME, c);
        Encode::encode(&self.public_e, c);
        Encode::encode(&self.public_n, c);
    }
}

impl<'a> DecodeRef<'a> for SshRsaPublicKey {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?; // TODO: use
        let _: &str = DecodeRef::decode(c).filter(|x| *x == <SshRsa as SignatureAlgorithm>::NAME)?;
        let e = DecodeRef::decode(c)?;
        let n = DecodeRef::decode(c)?;
        Some(SshRsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}
