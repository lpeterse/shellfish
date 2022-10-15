use super::super::method::*;
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

#[derive(Clone, Debug)]
pub struct MsgUserAuthRequest_ {
    pub user_name: String,
    pub service_name: String,
    pub method_name: String,
    pub method_blob: Vec<u8>
}

impl<'a> Message for MsgUserAuthRequest_ {
    const NUMBER: u8 = 50;
}

impl SshDecode for MsgUserAuthRequest_ {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let user_name = d.take_str_framed()?.into();
        let service_name = d.take_str_framed()?.into();
        let method_name = d.take_str_framed()?.into();
        let method_blob = d.take_bytes_all()?.into();
        Some(Self {
            user_name,
            service_name,
            method_name,
            method_blob,
        })
    }
}
