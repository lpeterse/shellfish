use super::cert::*;
use super::ssh_ed25519::*;
use super::ssh_ed25519_cert::*;
use crate::util::codec::*;

/// A user or host identity.
///
/// This is either just a key or a certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct Identity(Vec<u8>);

/// A user or host public key.
pub type PublicKey = Identity;

impl Identity {
    pub fn algorithm(&self) -> &str {
        RefDecoder::new(&self.0).take_str_framed().unwrap_or("")
    }

    pub fn as_ssh_ed25519(&self) -> Option<SshEd25519PublicKey> {
        SshCodec::decode(&self.0).ok()
    }

    pub fn as_ssh_ed25519_cert(&self) -> Option<SshEd25519Cert> {
        SshCodec::decode(&self.0).ok()
    }

    pub fn as_cert(&self) -> Option<Box<dyn Cert>> {
        if let Some(cert) = self.as_ssh_ed25519_cert() {
            Some(Box::new(cert))
        } else {
            None
        }
    }
}

impl From<Vec<u8>> for Identity {
    fn from(x: Vec<u8>) -> Self {
        Self(x)
    }
}

impl SshEncode for Identity {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bytes_framed(&self.0)
    }
}

impl SshDecode for Identity {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        let len = d.take_u32be()?;
        let bytes = d.take_bytes(len as usize)?;
        Some(Self(Vec::from(bytes)))
    }
}
