use crate::transport::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgUserAuthBanner {
    message: String,
    language: String
}

impl MsgUserAuthBanner {
    pub fn new(message: String) -> Self {
        Self {
            message,
            language: String::new(),
        }
    }
}

impl Message for MsgUserAuthBanner {
    const NUMBER: u8 = 53;
}

impl SshEncode for MsgUserAuthBanner {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_str_framed(&self.message)?;
        e.push_str_framed(&self.language)?;
        Some(())
    }
}

impl SshDecode for MsgUserAuthBanner {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let message = d.take_str_framed()?.into();
        let language = d.take_str_framed()?.into();
        Some(Self {
            message,
            language,
        })
    }
}
