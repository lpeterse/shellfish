use super::certificate::*;
use super::ssh_ed25519::*;
use super::ssh_ed25519_cert::*;
use crate::util::codec::*;

/// A user or host identity.
///
/// This is either just a key or a certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct Identity(Vec<u8>);
pub type PublicKey = Identity;

impl Identity {
    pub fn algorithm(&self) -> &str {
        let f = || {
            let mut d = SliceDecoder::new(&self.0);
            let len = d.take_u32be()?;
            d.take_str(len as usize)
        };
        f().unwrap_or("")
    }

    // TODO
    pub fn is_valid_certificate(&self) -> bool {
        true
    }

    pub fn is_certificate(&self) -> bool {
        true
    }

    pub fn as_ssh_ed25519(&self) -> Option<SshEd25519PublicKey> {
        SliceDecoder::decode(&self.0)
    }

    pub fn as_ssh_ed25519_cert(&self) -> Option<SshEd25519Cert> {
        SliceDecoder::decode(&self.0)
    }

    pub fn as_cert(&self) -> Option<Box<dyn Cert>> {
        if let Some(cert) = self.as_ssh_ed25519_cert() {
            return Some(Box::new(cert));
        }
        None
    }
}

impl From<Vec<u8>> for Identity {
    fn from(x: Vec<u8>) -> Self {
        Self(x)
    }
}

impl Encode for Identity {
    fn size(&self) -> usize {
        4 + self.0.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(self.0.len() as u32)?;
        e.push_bytes(&self.0)?;
        Some(())
    }
}

impl Decode for Identity {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let len = d.take_u32be()?;
        let bytes = d.take_bytes(len as usize)?;
        Some(Self(Vec::from(bytes)))
    }
}
