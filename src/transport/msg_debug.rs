use crate::codec::*;
use crate::language::*;

#[derive(Clone, Debug)]
pub struct MsgDebug {
    always_display: bool,
    message: String,
    language: Language,
}

impl MsgDebug {
    const MSG_NUMBER: u8 = 4;
}

impl Encode for MsgDebug {
    fn size(&self) -> usize {
        1 + 1
            + Encode::size(&self.message)
            + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        c.push_u8(self.always_display as u8);
        Encode::encode(&self.message, c);
        Encode::encode(&self.language, c);
    }
}

impl<'a> Decode<'a> for MsgDebug {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            always_display: d.take_u8()? != 0,
            message: Decode::decode(d)?,
            language: Decode::decode(d)?,
        }
        .into()
    }
}
