use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Ignore {
    data: Vec<u8>
}

impl Ignore {
    const MSG_NUMBER: u8 = 2;
}

impl<'a> Codec<'a> for Ignore {
    fn size(&self) -> usize {
        1 + Codec::size(&self.data)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.data, c);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            data: Codec::decode(d)?,
        }
        .into()
    }
}
