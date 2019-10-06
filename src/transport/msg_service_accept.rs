use crate::codec::*;
use super::message::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgServiceAccept<'a>(&'a str);

impl<'a> Message for MsgServiceAccept<'a> {
    const NUMBER: u8 = 6;
}

impl<'a> Encode for MsgServiceAccept<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.0, c);
    }
}

impl<'a> DecodeRef<'a> for MsgServiceAccept<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self(DecodeRef::decode(d)?).into()
    }
}
