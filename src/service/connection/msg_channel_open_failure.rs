use crate::language::*;
use crate::codec::*;

pub struct MsgChannelOpenFailure {
    pub recipient_channel: u32,
    pub reason_code: u32,
    pub description: String,
    pub language: Language,
}

pub struct ChannelOpenFailureReason (u32);

impl ChannelOpenFailureReason {
    pub const ADMINISTRATIVELY_PROHIBITED: Self = Self(1);
    pub const OPEN_CONNECT_FAILED : Self = Self(2);
    pub const UNKNOWN_CHANNEL_TYPE: Self = Self(3);
    pub const RESOURCE_SHORTAGE: Self = Self(4);
}

impl<'a> MsgChannelOpenFailure {
    const MSG_NUMBER: u8 = 92;
}

impl Encode for MsgChannelOpenFailure {
    fn size(&self) -> usize {
        1 + 4 + 4
        + Encode::size(&self.description)
        + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.reason_code);
        Encode::encode(&self.description, e);
        Encode::encode(&self.language, e);
    }
}

impl<'a> Decode<'a> for MsgChannelOpenFailure {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            reason_code: d.take_u32be()?,
            description: Decode::decode(d)?,
            language: Decode::decode(d)?,
        }.into()
    }
}
