mod ssh_ed25519;
mod ssh_ed25519_cert;
mod ssh_rsa;

pub use self::ssh_ed25519::*;
pub use self::ssh_ed25519_cert::*;
pub use self::ssh_rsa::*;

use crate::codec::*;

pub trait AuthenticationAlgorithm {
    type Identity;
    type Signature;
    type SignatureFlags: Copy + Default + Into<u32>;

    const NAME: &'static str;
}

#[derive(Clone, Debug, PartialEq)]
pub enum HostIdentity {
    Ed25519Key(<SshEd25519 as AuthenticationAlgorithm>::Identity),
    Ed25519Cert(<SshEd25519Cert as AuthenticationAlgorithm>::Identity),
    RsaKey(<SshRsa as AuthenticationAlgorithm>::Identity),
    Unknown(String),
}

impl HostIdentity {
    pub fn algorithm(&self) -> &str {
        match self {
            Self::Ed25519Key(_) => <SshEd25519 as AuthenticationAlgorithm>::NAME,
            Self::Ed25519Cert(_) => <SshEd25519Cert as AuthenticationAlgorithm>::NAME,
            Self::RsaKey(_) => <SshRsa as AuthenticationAlgorithm>::NAME,
            Self::Unknown(x) => x.as_str(),
        }
    }
}

impl Encode for HostIdentity {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519Key(k) => k.size(),
            Self::Ed25519Cert(k) => k.size(),
            Self::RsaKey(k) => k.size(),
            Self::Unknown(algo) => Encode::size(&algo),
        }
    }
    fn encode<E: Encoder>(&self,c: &mut E) {
        match self {
            Self::Ed25519Key(k) => {
                Encode::encode(k, c);
            },
            Self::Ed25519Cert(k) => {
                Encode::encode(k, c);
            },
            Self::RsaKey(k) => {
                Encode::encode(k, c);
            },
            Self::Unknown(algo) => {
                Encode::encode(&algo, c);
            }
        }
    }
}

impl Decode for HostIdentity {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        None.or_else(|| {
                let mut d_ = d.clone();
                let r = DecodeRef::decode(&mut d_).map(Self::Ed25519Key);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = DecodeRef::decode(&mut d_).map(Self::Ed25519Cert);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = DecodeRef::decode(&mut d_).map(Self::RsaKey);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = Decode::decode(&mut d_).map(Self::Unknown);
                if r.is_some() { *d = d_ };
                r
            })
    }
}

#[derive(Clone, Debug)]
pub enum HostIdentitySignature {
    Ed25519Signature(<SshEd25519 as AuthenticationAlgorithm>::Signature),
}

impl Encode for HostIdentitySignature {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519Signature(k) => k.size(),
        }
    }
    fn encode<E: Encoder>(&self,c: &mut E) {
        match self {
            Self::Ed25519Signature(k) => {
                Encode::encode(k, c);
            },
        }
    }
}

impl <'a> DecodeRef<'a> for HostIdentitySignature {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Some(Self::Ed25519Signature(DecodeRef::decode(d)?))
    }
}
