use crate::codec::*;
use crate::keys::*;

#[derive(Clone, Debug)]
pub struct MsgSignRequest<'a> {
    pub key: PublicKey,
    pub data: &'a [u8],
    pub flags: u32,
}

impl <'a> MsgSignRequest<'a> {
    pub const MSG_NUMBER: u8 = 13;
}

impl <'a> Codec<'a> for MsgSignRequest<'a> {
    fn size(&self) -> usize {
        1 + Codec::size(&self.key)
        + Codec::size(&self.data)
        + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.key, e);
        Codec::encode(&self.data, e);
        e.push_u32be(self.flags);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            key: Codec::decode(d)?,
            data: Codec::decode(d)?,
            flags: d.take_u32be()?,
        }.into()
    }
}
