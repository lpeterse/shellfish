use crate::codec::*;

#[derive(Clone, Debug)]
pub struct MsgIgnore<'a> {
    data: &'a [u8]
}

impl <'a> MsgIgnore<'a> {
    const MSG_NUMBER: u8 = 2;
}

impl <'a> Encode for MsgIgnore<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.data)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.data, c);
    }
}

impl<'a> DecodeRef<'a> for MsgIgnore<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            data: DecodeRef::decode(d)?,
        }
        .into()
    }
}
