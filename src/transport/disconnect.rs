mod reason;

pub use self::reason::*;

use crate::codec::*;
use crate::language::*;

#[derive(Clone, Debug)]
pub struct Disconnect {
    reason: Reason,
    description: String,
    language: Language,
}

impl Disconnect {
    const MSG_NUMBER: u8 = 1;
}

impl<'a> Codec<'a> for Disconnect {
    fn size(&self) -> usize {
        1 + Codec::size(&self.reason) + Codec::size(&self.description) + Codec::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.reason, c);
        Codec::encode(&self.description, c);
        Codec::encode(&self.language, c);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            reason: Codec::decode(d)?,
            description: Codec::decode(d)?,
            language: Codec::decode(d)?,
        }.into()
    }
}
