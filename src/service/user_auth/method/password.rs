use super::*;

#[derive(Debug)]
pub struct PasswordMethod(pub String);

impl <'a> Method<'a> for PasswordMethod {
    const NAME: &'static str = "password";
}

impl <'a> Codec<'a> for PasswordMethod {
    fn size(&self) -> usize {
        Codec::size(&self.0)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Codec::encode(&self.0, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Codec::decode(d).map(PasswordMethod)
    }
}