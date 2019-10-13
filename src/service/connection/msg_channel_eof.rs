use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub struct MsgChannelEof {
    pub recipient_channel: u32,
}

impl Message for MsgChannelEof {
    const NUMBER: u8 = 96;
}

impl Encode for MsgChannelEof {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        e.push_u32be(self.recipient_channel);
    }
}

impl Decode for MsgChannelEof {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?
        }.into()
    }
}
