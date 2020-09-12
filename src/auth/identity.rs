use super::ssh_ed25519::*;
use super::ssh_ed25519_cert::*;
use super::ssh_rsa::*;
use super::*;
use crate::util::codec::*;

/// A user or host identity.
///
/// This is either just a key or a certificate.
#[derive(Clone, Debug, PartialEq)]
pub enum Identity {
    Ed25519PublicKey(Ed25519PublicKey),
    Ed25519Certificate(Ed25519Certificate),
    RsaPublicKey(RsaPublicKey),
    Other(String),
}

impl Identity {
    /// The associated algorithm's name.
    pub fn algorithm(&self) -> &str {
        match self {
            Self::Ed25519PublicKey(_) => SshEd25519::NAME,
            Self::Ed25519Certificate(_) => SshEd25519Cert::NAME,
            Self::RsaPublicKey(_) => SshRsa::NAME,
            Self::Other(x) => x.as_str(),
        }
    }

    pub fn public_key_equals(&self, public_key: &Box<dyn PublicKey>) -> bool {
        match self {
            Self::Ed25519PublicKey(x) => x.equals(public_key),
            Self::RsaPublicKey(x) => x.equals(public_key),
            _ => false,
        }
    }

    pub fn verify_signature(&self, signature: &Signature, data: &[u8]) -> bool {
        match self {
            Self::Ed25519PublicKey(x) => x.verify_signature(&signature, &data),
            Self::RsaPublicKey(x) => x.verify_signature(&signature, &data),
            _ => false,
        }
    }
}

impl Encode for Identity {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519PublicKey(x) => 4 + Encode::size(x),
            Self::Ed25519Certificate(x) => 4 + Encode::size(x),
            Self::RsaPublicKey(x) => 4 + Encode::size(x),
            Self::Other(_) => 0,
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        match self {
            Self::Ed25519PublicKey(x) => {
                e.push_u32be(x.size() as u32)?;
                Encode::encode(x, e)
            }
            Self::Ed25519Certificate(x) => {
                e.push_u32be(x.size() as u32)?;
                Encode::encode(x, e)
            }
            Self::RsaPublicKey(x) => {
                e.push_u32be(x.size() as u32)?;
                Encode::encode(x, e)
            }
            Self::Other(_) => None,
        }
    }
}

impl Decode for Identity {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let mut d_ = d.clone();
        if let Some(r) = d_.isolate_u32be(|x| DecodeRef::decode(x)) {
            *d = d_;
            return Some(Self::Ed25519PublicKey(r));
        }
        let mut d_ = d.clone();
        if let Some(r) = d_.isolate_u32be(|x| DecodeRef::decode(x)) {
            *d = d_;
            return Some(Self::Ed25519Certificate(r));
        }
        let mut d_ = d.clone();
        if let Some(r) = d_.isolate_u32be(|x| DecodeRef::decode(x)) {
            *d = d_;
            return Some(Self::RsaPublicKey(r));
        }
        d.isolate_u32be(|x| {
            let algo = Decode::decode(x)?;
            let _ = x.take_all()?;
            Some(Self::Other(algo))
        })
    }
}
