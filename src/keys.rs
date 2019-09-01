use crate::codec::*;
use crate::codec_ssh::*;

pub use self::curve25519::*;
pub use self::ed25519::*;
pub use self::rsa::*;
pub use self::unknown::*;

mod curve25519;
mod ed25519;
mod rsa;
mod unknown;

#[derive(Clone, Debug)]
pub enum PublicKey {
    Ed25519PublicKey(Ed25519PublicKey),
    RsaPublicKey(RsaPublicKey),
    UnknownPublicKey(UnknownPublicKey),
}

impl <'a> SshCodec<'a> for PublicKey {
    fn size(&self) -> usize {
        4 + match self {
            PublicKey::Ed25519PublicKey(k) => SshCodec::size(&"ssh-ed25519") + k.size(),
            PublicKey::RsaPublicKey(k) => SshCodec::size(&"ssh-rsa") + k.size(),
            PublicKey::UnknownPublicKey(k) => SshCodec::size(&k.algo) + k.key.len(),
        }
    }
    fn encode(&self,c: &mut Encoder<'a>) {
        c.push_u32be((self.size() - 4) as u32);
        match self {
            PublicKey::Ed25519PublicKey(k) => {
                SshCodec::encode(&"ssh-ed25519", c);
                SshCodec::encode(k, c);
            },
            PublicKey::RsaPublicKey(k) => {
                SshCodec::encode(&"ssh-rsa", c);
                SshCodec::encode(k, c);
            },
            PublicKey::UnknownPublicKey(k) => {
                SshCodec::encode(&k.algo, c);
                c.push_bytes(k.key.as_slice());
            }
        }
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        let mut dec = c.take_decoder(len as usize)?;
        Some(match SshCodec::decode(&mut dec)? {
            "ssh-ed25519" => PublicKey::Ed25519PublicKey(SshCodec::decode(&mut dec)?),
            "ssh-rsa" => PublicKey::RsaPublicKey(SshCodec::decode(&mut dec)?),
            algo => PublicKey::UnknownPublicKey(UnknownPublicKey {
                algo: String::from(algo),
                key:  Vec::from(dec.take_all()?),
            }),
        })
    }
}


#[derive(Clone, Debug)]
pub enum Signature {
    Ed25519Signature(Ed25519Signature),
    UnknownSignature(UnknownSignature),
}

impl <'a> SshCodec<'a> for Signature {
    fn size(&self) -> usize {
        4 + match self {
            Signature::Ed25519Signature(k) => SshCodec::size(&"ssh-ed25519") + k.size(),
            Signature::UnknownSignature(k) => SshCodec::size(&k.algo) + k.signature.len(),
        }
    }
    fn encode(&self,c: &mut Encoder<'a>) {
        c.push_u32be((self.size() - 4) as u32);
        match self {
            Signature::Ed25519Signature(k) => {
                SshCodec::encode(&"ssh-ed25519", c);
                SshCodec::encode(k, c);
            },
            Signature::UnknownSignature(k) => {
                SshCodec::encode(&k.algo, c);
                c.push_bytes(k.signature.as_slice());
            }
        }
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        let mut dec = c.take_decoder(len as usize)?;
        Some(match SshCodec::decode(&mut dec)? {
            "ssh-ed25519" => Signature::Ed25519Signature(SshCodec::decode(&mut dec)?),
            algo => Signature::UnknownSignature(UnknownSignature {
                algo: String::from(algo),
                signature:  Vec::from(dec.take_all()?),
            }),
        })
    }
}
