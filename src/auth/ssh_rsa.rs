use super::*;
use crate::util::codec::*;

use std::any::Any;

pub struct SshRsa {}

impl SshRsa {
    pub const NAME: &'static str = "ssh-rsa";
}

#[derive(PartialEq, Clone, Debug)]
pub struct RsaPublicKey {
    pub public_e: Vec<u8>,
    pub public_n: Vec<u8>,
}

/*
impl PublicKey for RsaPublicKey {
    fn algorithm(&self) -> &str {
        SshRsa::NAME
    }

    fn verify_signature(&self, _signature: &Signature, _data: &[u8]) -> bool {
        todo!("IMPLEMENT RSA SIGNATURE VERIFICATION")
    }

    fn equals(&self, _public_key: &Box<dyn PublicKey>) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}*/

impl Encode for RsaPublicKey {
    fn size(&self) -> usize {
        8 + Encode::size(&SshRsa::NAME) + self.public_e.len() + self.public_n.len()
    }

    fn encode<E: Encoder>(&self, c: &mut E) -> Option<()> {
        Encode::encode(&SshRsa::NAME, c)?;
        c.push_u32be(self.public_e.len() as u32)?;
        c.push_bytes(&self.public_e)?;
        c.push_u32be(self.public_n.len() as u32)?;
        c.push_bytes(&self.public_n)
    }
}

impl<'a> DecodeRef<'a> for RsaPublicKey {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let _: &str = DecodeRef::decode(c).filter(|x| *x == SshRsa::NAME)?;
        let l = c.take_u32be()?;
        let e = Vec::from(c.take_bytes(l as usize)?);
        let l = c.take_u32be()?;
        let n = Vec::from(c.take_bytes(l as usize)?);
        Some(RsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}
