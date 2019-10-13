use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub struct MsgRequestFailure;

impl Message for MsgRequestFailure {
    const NUMBER: u8 = 82;
}

impl Encode for MsgRequestFailure {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
    }
}

impl Decode for MsgRequestFailure {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self.into()
    }
}
