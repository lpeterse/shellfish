use super::*;

#[derive(Debug)]
pub struct PasswordMethod(pub String);

impl <'a> AuthMethod for PasswordMethod {
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

impl <'a> DecodeRef<'a> for PasswordMethod {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        DecodeRef::decode(d).map(PasswordMethod)
    }
}
