use crate::codec::*;
use crate::algorithm::*;

#[derive(Clone, Debug)]
pub struct MsgSignRequest<'a, S: SignatureAlgorithm, D: Encode> {
    pub key: &'a S::PublicKey,
    pub data: &'a D,
    pub flags: u32,
}

impl <'a, S: SignatureAlgorithm,D: Encode> MsgSignRequest<'a,S,D> {
    pub const MSG_NUMBER: u8 = 13;
}

impl <'a, S: SignatureAlgorithm, D: Encode> Encode for MsgSignRequest<'a,S,D>
where
    S::PublicKey: Encode,
    S::Signature: Encode,
{
    fn size(&self) -> usize {
        1 + Encode::size(self.key)
        + 4
        + Encode::size(self.data)
        + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(self.key, e);
        e.push_u32be(Encode::size(self.data) as u32);
        Encode::encode(self.data, e);
        e.push_u32be(self.flags);
    }
}
