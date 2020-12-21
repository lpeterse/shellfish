use super::method::*;
use crate::transport::Message;
use crate::util::codec::*;

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
        13 + self.user_name.len() + self.service_name.len() + M::NAME.len() + self.method.size()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push_str_framed(&self.user_name)?;
        e.push_str_framed(&self.service_name)?;
        e.push_str_framed(M::NAME)?;
        e.push(&self.method)
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
