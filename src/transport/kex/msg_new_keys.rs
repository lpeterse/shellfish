use super::*;
use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgNewKeys {}

impl MsgNewKeys {
    pub fn new() -> Self {
        Self {}
    }
}

impl Message for MsgNewKeys {
    const NUMBER: u8 = 21;
}

impl Encode for MsgNewKeys {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
    }
}

impl Decode for MsgNewKeys {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {})
    }
}
