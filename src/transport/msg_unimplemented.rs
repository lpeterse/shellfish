use crate::codec::*;
use crate::message::*;

#[derive(Copy, Clone, Debug)]
pub struct MsgUnimplemented {
    pub packet_number: u32
}

impl Message for MsgUnimplemented {
    const NUMBER: u8 = 3;
}

impl Encode for MsgUnimplemented {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        c.push_u32be(self.packet_number);
    }
}

impl Decode for MsgUnimplemented {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            packet_number: d.take_u32be()?,
        }
        .into()
    }
}
