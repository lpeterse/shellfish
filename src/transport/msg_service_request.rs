use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgServiceRequest<'a> (pub &'a str);

impl <'a> MsgServiceRequest<'a> {
    const MSG_NUMBER: u8 = 5;
}

impl <'a> Encode for MsgServiceRequest<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.0, c);
    }
}

impl<'a> Decode<'a> for MsgServiceRequest<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self (Decode::decode(d)?).into()
    }
}
