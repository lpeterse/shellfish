use super::method::*;
use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Request<'a, M: Method<'a>> {
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub method: M,
}

impl<'a, M: Method<'a>> Request<'a, M> {
    const MSG_NUMBER: u8 = 50;
}

impl<'a, M: Method<'a>> Codec<'a> for Request<'a, M> {
    fn size(&self) -> usize {
        1 + Codec::size(&self.user_name)
            + Codec::size(&self.service_name)
            + Codec::size(&M::NAME)
            + Codec::size(&self.method)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.user_name, e);
        Codec::encode(&self.service_name, e);
        Codec::encode(&M::NAME, e);
        Codec::encode(&self.method, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        let user_name = Codec::decode(d)?;
        let service_name = Codec::decode(d)?;
        let _: &str = Codec::decode(d).filter(|x| *x == M::NAME)?;
        Request {
            user_name,
            service_name,
            method: Codec::decode(d)?,
        }
        .into()
    }
}
