use super::*;
use crate::codec::*;

pub struct SshRsa {}

impl AuthAlgorithm for SshRsa {
    type AuthIdentity = SshRsaPublicKey;
    type AuthSignature = ();
    type AuthSignatureFlags = SshRsaSignatureFlags;

    const NAME: &'static str = "ssh-rsa";
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshRsaPublicKey {
    pub public_e: Vec<u8>,
    pub public_n: Vec<u8>,
}

impl Encode for SshRsaPublicKey {
    fn size(&self) -> usize {
        4 + Encode::size(&<SshRsa as AuthAlgorithm>::NAME)
            + 4 + self.public_e.len()
            + 4 + self.public_n.len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be((Encode::size(self) - 4) as u32);
        Encode::encode(&<SshRsa as AuthAlgorithm>::NAME, c);
        c.push_u32be(self.public_e.len() as u32);
        c.push_bytes(&self.public_e);
        c.push_u32be(self.public_n.len() as u32);
        c.push_bytes(&self.public_n);
    }
}

impl<'a> DecodeRef<'a> for SshRsaPublicKey {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let _len = c.take_u32be()?; // TODO: use
        let _: &str =
            DecodeRef::decode(c).filter(|x| *x == <SshRsa as AuthAlgorithm>::NAME)?;
        let l = c.take_u32be()?;
        let e = Vec::from(c.take_bytes(l as usize)?);
        let l = c.take_u32be()?;
        let n = Vec::from(c.take_bytes(l as usize)?);
        Some(SshRsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SshRsaSignatureFlags {
    // SSH_AGENT_RSA_SHA2_256
    // SSH_AGENT_RSA_SHA2_512
}

impl Default for SshRsaSignatureFlags {
    fn default() -> Self {
        Self {}
    }
}

impl Into<u32> for SshRsaSignatureFlags {
    fn into(self) -> u32 {
        0
    }
}
