use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgChannelWindowAdjust {
    pub recipient_channel: u32,
    pub bytes_to_add: u32,
}

impl Message for MsgChannelWindowAdjust {
    const NUMBER: u8 = 93;
}

impl  Encode for MsgChannelWindowAdjust {
    fn size(&self) -> usize {
        1 + 4 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.bytes_to_add);
    }
}

impl Decode for MsgChannelWindowAdjust {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        MsgChannelWindowAdjust {
            recipient_channel: d.take_u32be()?,
            bytes_to_add: d.take_u32be()?,
        }.into()
    }
}
