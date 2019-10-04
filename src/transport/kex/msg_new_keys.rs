use crate::codec::*;

#[derive(Clone, Debug)]
pub struct NewKeys {}

impl NewKeys {
    const MSG_NUMBER: u8 = 21;

    pub fn new() -> Self {
        Self {}
    }
}

impl Encode for NewKeys {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER);
    }
}

impl Decode for NewKeys {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(Self::MSG_NUMBER)?;
        Some(Self {})
    }
}
