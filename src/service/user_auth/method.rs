use crate::codec::*;

#[derive(Clone, Debug)]
pub enum Method {
    Password(Password),
}

#[derive(Clone, Debug)]
pub struct Password(pub String);

impl Password {
    const METHOD_NAME: &'static str = "password";
}

impl<'a> Codec<'a> for Method {
    fn size(&self) -> usize {
        match self {
            Self::Password(x) => Codec::size(&Password::METHOD_NAME) + 1 + Codec::size(&x.0),
        }
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        match self {
            Self::Password(x) => {
                Codec::encode(&Password::METHOD_NAME, c);
                c.push_u8(0);
                Codec::encode(&x.0, c);
            }
        }
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let method: String = Codec::decode(d)?;
        if method == Password::METHOD_NAME {
            d.take_u8().filter(|x| *x == 0)?;
            return Some(Self::Password(Password(Codec::decode(d)?)));
        }
        None
    }
}
