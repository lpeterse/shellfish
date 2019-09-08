use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ServiceAccept<'a> (&'a str);

impl <'a> ServiceAccept<'a> {
    const MSG_NUMBER: u8 = 6;
}

impl<'a> Codec<'a> for ServiceAccept<'a> {
    fn size(&self) -> usize {
        1 + Codec::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.0, c);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self (Codec::decode(d)?).into()
    }
}
