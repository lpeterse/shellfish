use super::method::*;
use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgUserAuthRequest<'a, M: AuthMethod> {
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub method: M,
}

impl<'a, M: AuthMethod> Message for MsgUserAuthRequest<'a, M> {
    const NUMBER: u8 = 50;
}

impl<'a, M: AuthMethod + Encode> Encode for MsgUserAuthRequest<'a, M> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.user_name)
            + Encode::size(&self.service_name)
            + Encode::size(&M::NAME)
            + Encode::size(&self.method)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        Encode::encode(&self.user_name, e);
        Encode::encode(&self.service_name, e);
        Encode::encode(&M::NAME, e);
        Encode::encode(&self.method, e);
    }
}

impl<'a, M: AuthMethod + DecodeRef<'a>> DecodeRef<'a> for MsgUserAuthRequest<'a, M> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let user_name = DecodeRef::decode(d)?;
        let service_name = DecodeRef::decode(d)?;
        let _: &str = DecodeRef::decode(d).filter(|x| *x == M::NAME)?;
        MsgUserAuthRequest {
            user_name,
            service_name,
            method: DecodeRef::decode(d)?,
        }
        .into()
    }
}
