use crate::util::codec::*;

pub struct SshRsa {}

impl SshRsa {
    pub const NAME: &'static str = "ssh-rsa";
}

#[derive(PartialEq, Clone, Debug)]
pub struct RsaPublicKey {
    pub public_e: Vec<u8>,
    pub public_n: Vec<u8>,
}

impl Encode for RsaPublicKey {
    fn size(&self) -> usize {
        12 + SshRsa::NAME.len() + self.public_e.len() + self.public_n.len()
    }

    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(SshRsa::NAME)?;
        e.push_bytes_framed(&self.public_e)?;
        e.push_bytes_framed(&self.public_n)
    }
}

impl<'a> DecodeRef<'a> for RsaPublicKey {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let _: &str = DecodeRef::decode(c).filter(|x| *x == SshRsa::NAME)?;
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
