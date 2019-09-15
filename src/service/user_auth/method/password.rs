use super::*;

#[derive(Debug)]
pub struct PasswordMethod(pub String);

impl <'a> Method for PasswordMethod {
    const NAME: &'static str = "password";
}

impl Encode for PasswordMethod {
    fn size(&self) -> usize {
        Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.0, e);
    }
}

impl <'a> Decode<'a> for PasswordMethod {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Decode::decode(d).map(PasswordMethod)
    }
}
