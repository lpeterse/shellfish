use super::*;
use crate::keys::*;

#[derive(Debug)]
pub struct Pubkey<'a> {
    pub algorithm: &'a str,
    pub public_key: PublicKey,
    pub signature: Option<&'a [u8]>,
}

impl<'a> Method<'a> for Pubkey<'a> {
    const NAME: &'static str = "publickey";
}

impl<'a> Codec<'a> for Pubkey<'a> {
    fn size(&self) -> usize {
        Codec::size(&self.algorithm)
            + Codec::size(&self.public_key)
            + 1
            + self.signature.map(|x| Codec::size(&x)).unwrap_or(0)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Codec::encode(&self.algorithm, e);
        Codec::encode(&self.public_key, e);
        e.push_u8(self.signature.is_some() as u8);
        self.signature.map(|x| Codec::encode(&x, e)).unwrap_or(());
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let algorithm = Codec::decode(d)?;
        let public_key = Codec::decode(d)?;
        let signature = if d.take_u8()? != 0 {
                Some(Codec::decode(d)?)
            } else {
                None
            };
        Pubkey {
            algorithm,
            public_key,
            signature,
        }
        .into()
    }
}
