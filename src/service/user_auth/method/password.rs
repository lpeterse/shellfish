use super::*;

#[derive(Debug)]
pub struct Password(pub String);

impl <'a> Method<'a> for Password {
    const NAME: &'static str = "password";
}

impl <'a> Codec<'a> for Password {
    fn size(&self) -> usize {
        Codec::size(&self.0)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Codec::encode(&self.0, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Codec::decode(d).map(Password)
    }
}