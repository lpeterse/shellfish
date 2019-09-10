use crate::language::*;
use crate::codec::*;

pub struct MsgChannelOpenFailure {
    pub recipient_channel: u32,
    pub reason_code: u32,
    pub description: String,
    pub language: Language,
}

impl<'a> MsgChannelOpenFailure {
    const MSG_NUMBER: u8 = 91;
}

impl<'a> Codec<'a> for MsgChannelOpenFailure {
    fn size(&self) -> usize {
        1 + 4 + 4
        + Codec::size(&self.description)
        + Codec::size(&self.language)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.reason_code);
        Codec::encode(&self.description, e);
        Codec::encode(&self.language, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            reason_code: d.take_u32be()?,
            description: Codec::decode(d)?,
            language: Codec::decode(d)?,
        }.into()
    }
}
