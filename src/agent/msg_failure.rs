use crate::codec::*;

#[derive(Clone, Debug)]
pub struct MsgFailure {}

impl MsgFailure {
    pub const MSG_NUMBER: u8 = 5;
}

impl <'a> Codec<'a> for MsgFailure {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {}.into()
    }
}
