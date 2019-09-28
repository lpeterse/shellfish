use crate::codec::*;

#[derive(Clone, Debug)]
pub struct NewKeys {}

impl NewKeys {
    const MSG_NUMBER: u8 = 21;
}

impl Encode for NewKeys {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER);
    }
}

impl <'a> DecodeRef<'a> for NewKeys {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Some(Self {})
    }
}
