mod reason;

pub use self::reason::*;

use crate::codec::*;
use crate::language::*;

#[derive(Clone, Debug)]
pub struct MsgDisconnect {
    reason: Reason,
    description: String,
    language: Language,
}

impl MsgDisconnect {
    const MSG_NUMBER: u8 = 1;

    pub fn by_application(description: String) -> Self {
        Self { reason: Reason::ByApplication, description, language: Language::empty() }
    }
}

impl Encode for MsgDisconnect {
    fn size(&self) -> usize {
        1 + Encode::size(&self.reason) + Encode::size(&self.description) + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.reason, c);
        Encode::encode(&self.description, c);
        Encode::encode(&self.language, c);
    }
}

impl<'a> Decode<'a> for MsgDisconnect {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            reason: Decode::decode(d)?,
            description: Decode::decode(d)?,
            language: Decode::decode(d)?,
        }.into()
    }
}
