use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub struct MsgRequestSuccess<'a> {
    pub data: &'a [u8],
}

impl<'a> Message for MsgRequestSuccess<'a> {
    const NUMBER: u8 = 81;
}

impl <'a> Encode for MsgRequestSuccess<'a> {
    fn size(&self) -> usize {
        1 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        e.push_bytes(&self.data);
    }
}

impl<'a> DecodeRef<'a> for MsgRequestSuccess<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            data: d.take_all()?,
        }.into()
    }
}
