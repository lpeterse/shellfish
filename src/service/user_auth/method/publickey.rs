use super::*;
use crate::algorithm::*;

#[derive(Debug)]
pub struct PublicKeyMethod<S: SignatureAlgorithm> {
    pub public_key: S::PublicKey,
    pub signature: Option<S::Signature>,
}

impl<'a, S: SignatureAlgorithm> Method<'a> for PublicKeyMethod<S>
where
    S::PublicKey: Codec<'a>,
    S::Signature: Codec<'a>,
{
    const NAME: &'static str = "publickey";
}

impl<'a, S: SignatureAlgorithm> Codec<'a> for PublicKeyMethod<S>
where
    S::PublicKey: Codec<'a>,
    S::Signature: Codec<'a>,
{
    fn size(&self) -> usize {
        Codec::size(&S::NAME)
            + Codec::size(&self.public_key)
            + 1
            + match self.signature { None => 0, Some(ref x) => Codec::size(x) }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Codec::encode(&S::NAME, e);
        Codec::encode(&self.public_key, e);
        e.push_u8(self.signature.is_some() as u8);
        match self.signature { None => (), Some(ref x) => Codec::encode(x, e) }
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let _: &str = Codec::decode(d).filter(|x| *x == S::NAME)?;
        let public_key = Codec::decode(d)?;
        let signature = if d.take_u8()? != 0 {
            Some(Codec::decode(d)?)
        } else {
            None
        };
        PublicKeyMethod {
            public_key,
            signature,
        }
        .into()
    }
}
