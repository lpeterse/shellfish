use crate::codec::*;
use crate::keys::*;

#[derive(Clone, Debug)]
pub struct MsgSignResponse {
    pub signature: Signature,
}

impl MsgSignResponse {
    pub const MSG_NUMBER: u8 = 14;
}

impl <'a> Codec<'a> for MsgSignResponse {
    fn size(&self) -> usize {
        1 + Codec::size(&self.signature)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.signature, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            signature: Codec::decode(d)?
        }.into()
    }
}
