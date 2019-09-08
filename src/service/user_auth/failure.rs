use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Failure<'a> {
    methods: Vec<&'a str>,
    partial_success: bool,
}

impl <'a> Failure<'a> {
    const MSG_NUMBER: u8 = 51;
}

impl<'a> Codec<'a> for Failure<'a> {
    fn size(&self) -> usize {
        1 + NameList::size(&self.methods) + 1
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        NameList::encode(&self.methods, e);
        e.push_u8(self.partial_success as u8);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            methods: NameList::decode_str(d)?,
            partial_success: d.take_u8().map(|x| x != 0)?,
        }
        .into()
    }
}