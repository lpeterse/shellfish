use crate::codec::*;

#[derive(Debug)]
pub struct MsgChannelFailure {
    pub recipient_channel: u32,
}

impl MsgChannelFailure {
    const MSG_NUMBER: u8 = 100;
}

impl Encode for MsgChannelFailure {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER);
        e.push_u32be(self.recipient_channel);
    }
}

impl Decode for MsgChannelFailure {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?
        }.into()
    }
}
