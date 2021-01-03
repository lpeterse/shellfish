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

impl<'a, M: AuthMethod + SshEncode> SshEncode for MsgUserAuthRequest<'a, M> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_str_framed(&self.user_name)?;
        e.push_str_framed(&self.service_name)?;
        e.push_str_framed(M::NAME)?;
        e.push(&self.method)
    }
}

impl<'a, M: AuthMethod + SshDecodeRef<'a>> SshDecodeRef<'a> for MsgUserAuthRequest<'a, M> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let user_name = d.take_str_framed()?;
        let service_name = d.take_str_framed()?;
        d.expect_str_framed(M::NAME)?;
        let method = d.take()?;
        Some(Self {
            user_name,
            service_name,
            method,
        })
    }
}
