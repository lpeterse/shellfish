use crate::util::codec::*;

pub struct SshRsa;

impl SshRsa {
    pub const NAME: &'static str = "ssh-rsa";
}

#[derive(PartialEq, Clone, Debug)]
pub struct RsaPublicKey {
    pub public_e: Vec<u8>,
    pub public_n: Vec<u8>,
}

impl SshEncode for RsaPublicKey {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(SshRsa::NAME)?;
        e.push_bytes_framed(&self.public_e)?;
        e.push_bytes_framed(&self.public_n)
    }
}

impl<'a> SshDecodeRef<'a> for RsaPublicKey {
    fn decode<D: SshDecoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_str_framed(SshRsa::NAME)?;
        let l = c.take_u32be()?;
        let e = Vec::from(c.take_bytes(l as usize)?);
        let l = c.take_u32be()?;
        let n = Vec::from(c.take_bytes(l as usize)?);
        Some(RsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}
