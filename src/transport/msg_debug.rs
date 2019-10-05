use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgDebug<'a> {
    pub always_display: bool,
    pub message: &'a str,
    pub language: &'a str,
}

impl<'a> MsgDebug<'a> {
    pub fn new(message: &'a str, language: &'a str) -> Self {
        Self {
            always_display: true,
            message,
            language
        }
    }
}

impl<'a> Message for MsgDebug<'a> {
    const NUMBER: u8 = 4;
}

impl <'a> Encode for MsgDebug<'a> {
    fn size(&self) -> usize {
        1 + 1
            + Encode::size(&self.message)
            + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER as u8);
        c.push_u8(self.always_display as u8);
        Encode::encode(&self.message, c);
        Encode::encode(&self.language, c);
    }
}

impl<'a> DecodeRef<'a> for MsgDebug<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            always_display: d.take_u8()? != 0,
            message: DecodeRef::decode(d)?,
            language: DecodeRef::decode(d)?,
        }
        .into()
    }
}
