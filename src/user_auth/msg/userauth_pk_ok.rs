use crate::transport::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgUserAuthPkOk {
    pub pk_algorithm: String,
    pub pk_blob: Vec<u8>,
}

impl Message for MsgUserAuthPkOk {
    const NUMBER: u8 = 60;
}

impl SshEncode for MsgUserAuthPkOk {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_str_framed(&self.pk_algorithm)?;
        e.push_bytes_framed(&self.pk_blob)?;
        Some(())
    }
}

impl SshDecode for MsgUserAuthPkOk {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let pk_algorithm = d.take_str_framed()?.into();
        let pk_blob = d.take_bytes_framed()?.into();
        Some(Self {
            pk_algorithm,
            pk_blob,
        })
    }
}
