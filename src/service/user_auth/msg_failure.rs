use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgFailure<'a> {
    pub methods: Vec<&'a str>,
    pub partial_success: bool,
}

impl<'a> Message for MsgFailure<'a> {
    const NUMBER: u8 = 51;
}

impl<'a> Encode for MsgFailure<'a> {
    fn size(&self) -> usize {
        1 + NameList::size(&self.methods) + 1
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        NameList::encode(&self.methods, e);
        e.push_u8(self.partial_success as u8);
    }
}

impl<'a> DecodeRef<'a> for MsgFailure<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &<Self as Message>::NUMBER)?;
        Self {
            methods: NameList::decode_str(d)?,
            partial_success: d.take_u8().map(|x| x != 0)?,
        }
        .into()
    }
}
