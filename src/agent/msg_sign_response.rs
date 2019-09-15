use crate::codec::*;

use crate::algorithm::*;

#[derive(Debug)]
pub struct MsgSignResponse<S: SignatureAlgorithm> {
    pub signature: S::Signature,
}

impl <S: SignatureAlgorithm> MsgSignResponse<S> {
    pub const MSG_NUMBER: u8 = 14;

    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self>
    where
        S::Signature: Decode<'a>
    {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            signature: Decode::decode(d)?
        }.into()
    }
}

impl <S: SignatureAlgorithm> Encode for MsgSignResponse<S>
where
    S::Signature: Encode
{
    fn size(&self) -> usize {
        1 + Encode::size(&self.signature)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.signature, e);
    }
}

impl <'a, S: SignatureAlgorithm> Decode<'a> for MsgSignResponse<S>
where
    S::Signature: Decode<'a>
{
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            signature: Decode::decode(d)?
        }.into()
    }
}
