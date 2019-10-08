use crate::codec::*;
use crate::transport::message::*;

#[derive(Clone, Debug)]
pub struct MsgSuccess {}

impl Message for MsgSuccess {
    const NUMBER: u8 = 52;
}

impl Encode for MsgSuccess {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER as u8);
    }
}

impl Decode for MsgSuccess {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {}.into()
    }
}