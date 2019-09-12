use crate::codec::*;
use crate::algorithm::*;

#[derive(Clone, Debug)]
pub struct MsgSignRequest<'a, S: SignatureAlgorithm> {
    pub key: S::PublicKey,
    pub data: &'a [u8],
    pub flags: u32,
}

impl <'a, S: SignatureAlgorithm> MsgSignRequest<'a,S> {
    pub const MSG_NUMBER: u8 = 13;
}

impl <'a, S: SignatureAlgorithm> MsgSignRequest<'a,S>
{
    pub fn size<'b>(&self) -> usize
    where
        S::PublicKey: Codec<'b>,
    {
        1 + Codec::size(&self.key)
        + Codec::size(&self.data)
        + 4
    }
    pub fn encode<'b, E: Encoder>(&self, e: &mut E)
    where
        S::PublicKey: Codec<'b>,
    {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.key, e);
        Codec::encode(&self.data, e);
        e.push_u32be(self.flags);
    }
    pub fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self>
    where
        S::PublicKey: Codec<'a>,
    {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            key: Codec::decode(d)?,
            data: Codec::decode(d)?,
            flags: d.take_u32be()?,
        }.into()
    }
}
