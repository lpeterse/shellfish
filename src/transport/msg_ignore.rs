use crate::codec::*;
use super::message::*;

#[derive(Clone, Debug)]
pub struct MsgIgnore<'a> {
    pub data: &'a [u8],
}

impl<'a> MsgIgnore<'a> {
    pub fn new() -> Self {
        Self { data: &[] }
    }
}

impl<'a> Message for MsgIgnore<'a> {
    const NUMBER: u8 = 2;
}

impl<'a> Encode for MsgIgnore<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.data)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.data, c);
    }
}

impl<'a> DecodeRef<'a> for MsgIgnore<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            data: DecodeRef::decode(d)?,
        }
        .into()
    }
}
