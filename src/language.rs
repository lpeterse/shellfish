use std::convert::TryFrom;
use crate::codec::*;

#[derive(Debug,Clone)]
pub struct Language (String);

impl AsRef<[u8]> for Language {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFrom<&[u8]> for Language {
    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self,Self::Error> {
        Ok(Self(String::from_utf8(Vec::from(x))?))
    }
}

impl<'a> Codec<'a> for Language {
    fn size(&self) -> usize {
        Codec::size(&self.0)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Codec::encode(&self.0, e)
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Codec::decode(d).map(Self)
    }
}
