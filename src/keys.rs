mod curve25519;
mod rsa;
mod unknown;

pub use self::curve25519::*;
pub use self::rsa::*;
pub use self::unknown::*;

use crate::codec::*;
use crate::algorithm::*;

#[derive(Clone, Debug)]
pub enum PublicKey {
    Ed25519PublicKey(<SshEd25519 as SignatureAlgorithm>::PublicKey),
    RsaPublicKey(RsaPublicKey),
    UnknownPublicKey(UnknownPublicKey),
}

impl <'a> Codec<'a> for PublicKey {
    fn size(&self) -> usize {
        4 + match self {
            PublicKey::Ed25519PublicKey(k) => k.size(),
            PublicKey::RsaPublicKey(k) => Codec::size(&"ssh-rsa") + k.size(),
            PublicKey::UnknownPublicKey(k) => Codec::size(&k.algo) + k.key.len(),
        }
    }
    fn encode<E: Encoder>(&self,c: &mut E) {
        c.push_u32be((self.size() - 4) as u32);
        match self {
            PublicKey::Ed25519PublicKey(k) => {
                Codec::encode(k, c);
            },
            PublicKey::RsaPublicKey(k) => {
                Codec::encode(&"ssh-rsa", c);
                Codec::encode(k, c);
            },
            PublicKey::UnknownPublicKey(k) => {
                Codec::encode(&k.algo, c);
                c.push_bytes(&k.key.as_slice());
            }
        }
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Some(PublicKey::Ed25519PublicKey(Codec::decode(d)?))
    }
}

// TODO: Should be removable..
#[derive(Clone, Debug)]
pub enum Signature {
    // TODO SshEd..
    Ed25519Signature(<SshEd25519 as SignatureAlgorithm>::Signature),
    UnknownSignature(UnknownSignature),
}

impl <'a> Codec<'a> for Signature {
    fn size(&self) -> usize {
        4 + match self {
            Signature::Ed25519Signature(k) => Codec::size(&"ssh-ed25519") + k.size(),
            Signature::UnknownSignature(k) => Codec::size(&k.algo) + k.signature.len(),
        }
    }
    fn encode<E: Encoder>(&self,c: &mut E) {
        c.push_u32be((self.size() - 4) as u32);
        match self {
            Signature::Ed25519Signature(k) => {
                Codec::encode(&"ssh-ed25519", c);
                Codec::encode(k, c);
            },
            Signature::UnknownSignature(k) => {
                Codec::encode(&k.algo, c);
                c.push_bytes(&k.signature.as_slice());
            }
        }
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        let mut dec = BDecoder(c.take_bytes(len as usize)?);
        Some(match Codec::decode(&mut dec)? {
            "ssh-ed25519" => Signature::Ed25519Signature(Codec::decode(&mut dec)?),
            algo => Signature::UnknownSignature(UnknownSignature {
                algo: String::from(algo),
                signature:  Vec::from(dec.take_all()?),
            }),
        })
    }
}
