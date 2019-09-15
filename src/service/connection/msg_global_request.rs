use crate::codec::*;

#[derive(Debug)]
pub struct MsgGlobalRequest<'a> {
    pub name: &'a str,
    pub want_reply: bool,
    pub data: &'a [u8],
}

impl<'a> MsgGlobalRequest<'a> {
    const MSG_NUMBER: u8 = 80;
}

impl <'a> Encode for MsgGlobalRequest<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.name) + 1 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER);
        Encode::encode(&self.name, e);
        e.push_u8(self.want_reply as u8);
        e.push_bytes(&self.data);
    }
}

impl<'a> Decode<'a> for MsgGlobalRequest<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Self {
            name: Decode::decode(d)?,
            want_reply: d.take_u8()? != 0,
            data: d.take_all()?,
        }.into()
    }
}
