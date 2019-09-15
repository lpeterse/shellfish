use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Success {}

impl Success {
    const MSG_NUMBER: u8 = 52;
}

impl Encode for Success {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
    }
}

impl<'a> Decode<'a> for Success {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {}.into()
    }
}