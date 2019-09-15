use super::method::*;
use crate::codec::*;

#[derive(Clone, Debug)]
pub struct MsgUserAuthRequest<'a, M: Method> {
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub method: M,
}

impl<'a, M: Method> MsgUserAuthRequest<'a, M> {
    pub const MSG_NUMBER: u8 = 50;
}

impl <'a, M: Method + Encode> Encode for MsgUserAuthRequest<'a, M> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.user_name)
            + Encode::size(&self.service_name)
            + Encode::size(&M::NAME)
            + Encode::size(&self.method)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.user_name, e);
        Encode::encode(&self.service_name, e);
        Encode::encode(&M::NAME, e);
        Encode::encode(&self.method, e);
    }
}

impl <'a, M: Method + Decode<'a>> Decode<'a> for MsgUserAuthRequest<'a, M> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        let user_name = Decode::decode(d)?;
        let service_name = Decode::decode(d)?;
        let _: &str = Decode::decode(d).filter(|x| *x == M::NAME)?;
        MsgUserAuthRequest {
            user_name,
            service_name,
            method: Decode::decode(d)?,
        }
        .into()
    }
}
