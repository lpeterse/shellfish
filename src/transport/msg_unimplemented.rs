use crate::codec::*;

#[derive(Clone, Debug)]
pub struct MsgUnimplemented {
    pub packet_number: u32
}

impl MsgUnimplemented {
    const MSG_NUMBER: u8 = 3;
}

impl Encode for MsgUnimplemented {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        c.push_u32be(self.packet_number);
    }
}

impl<'a> Decode<'a> for MsgUnimplemented {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            packet_number: d.take_u32be()?,
        }
        .into()
    }
}
