use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Unimplemented {
    packet_number: u32
}

impl Unimplemented {
    const MSG_NUMBER: u8 = 3;
}

impl<'a> Codec<'a> for Unimplemented {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        c.push_u32be(self.packet_number);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            packet_number: d.take_u32be()?,
        }
        .into()
    }
}
