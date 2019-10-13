use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgServiceRequest<'a>(pub &'a str);

impl<'a> Message for MsgServiceRequest<'a> {
    const NUMBER: u8 = 5;
}

impl<'a> Encode for MsgServiceRequest<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.0, c);
    }
}

impl<'a> DecodeRef<'a> for MsgServiceRequest<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self(DecodeRef::decode(d)?).into()
    }
}
