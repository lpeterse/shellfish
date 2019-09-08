use crate::codec::*;
use super::method::*;

#[derive(Clone, Debug)]
pub struct Request<'a> {
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub method: Method,
}

impl <'a> Request<'a> {
    const MSG_NUMBER: u8 = 50;
}

impl<'a> Codec<'a> for Request<'a> {
    fn size(&self) -> usize {
        1 + Codec::size(&self.user_name) + Codec::size(&self.service_name) + Codec::size(&self.method)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.user_name, c);
        Codec::encode(&self.service_name, c);
        Codec::encode(&self.method, c);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Request {
            user_name: Codec::decode(d)?,
            service_name: Codec::decode(d)?,
            method: Codec::decode(d)?,
        }
        .into()
    }
}
