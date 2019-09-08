use crate::codec::*;
use crate::language::*;

#[derive(Clone, Debug)]
pub struct Debug {
    always_display: bool,
    message: String,
    language: Language,
}

impl Debug {
    const MSG_NUMBER: u8 = 4;
}

impl<'a> Codec<'a> for Debug {
    fn size(&self) -> usize {
        1 + 1
            + Codec::size(&self.message)
            + Codec::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        c.push_u8(self.always_display as u8);
        Codec::encode(&self.message, c);
        Codec::encode(&self.language, c);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            always_display: d.take_u8()? != 0,
            message: Codec::decode(d)?,
            language: Codec::decode(d)?,
        }
        .into()
    }
}
