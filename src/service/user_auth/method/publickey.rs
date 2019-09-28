use super::*;
use crate::algorithm::*;

#[derive(Debug)]
pub struct PublicKeyMethod<S: SignatureAlgorithm> {
    pub public_key: S::PublicKey,
    pub signature: Option<S::Signature>,
}

impl<'a, S: SignatureAlgorithm> Method for PublicKeyMethod<S> {
    const NAME: &'static str = "publickey";
}

impl <S: SignatureAlgorithm> Encode for PublicKeyMethod<S>
where
    S::PublicKey: Encode,
    S::Signature: Encode,
{
    fn size(&self) -> usize {
        1 + Encode::size(&S::NAME)
            + Encode::size(&self.public_key)
            + match self.signature {
                None => 0,
                Some(ref x) => Encode::size(x),
            }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(self.signature.is_some() as u8);
        Encode::encode(&S::NAME, e);
        Encode::encode(&self.public_key, e);
        match self.signature {
            None => (),
            Some(ref x) => Encode::encode(x, e),
        }
    }
}

impl<'a, S: SignatureAlgorithm> DecodeRef<'a> for PublicKeyMethod<S>
where
    S::PublicKey: DecodeRef<'a>,
    S::Signature: DecodeRef<'a>,
{
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let b = d.take_u8()? != 0;
        let _: &str = DecodeRef::decode(d).filter(|x| *x == S::NAME)?;
        let public_key = DecodeRef::decode(d)?;
        let signature = if b {
            Some(DecodeRef::decode(d)?)
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
