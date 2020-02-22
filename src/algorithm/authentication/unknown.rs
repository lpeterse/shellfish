use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct UnknownIdentity {
    pub algo: String,
    pub data: Vec<u8>,
}

impl Encode for UnknownIdentity {
    fn size(&self) -> usize {
        Encode::size(&self.algo) + Encode::size(&self.data[..])
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.algo, e);
        Encode::encode(&self.data[..], e);
    }
}

impl Decode for UnknownIdentity {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self {
            algo: Decode::decode(d)?,
            data: DecodeRef::decode(d).map(|x: &'a [u8]| Vec::from(x))?,
        }
        .into()
    }
}
