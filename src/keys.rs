mod curve25519;
mod unknown;

pub use self::curve25519::*;
pub use self::unknown::*;

use crate::codec::*;
use crate::algorithm::*;

#[derive(Clone, Debug, PartialEq)]
pub enum PublicKey {
    Ed25519PublicKey(<SshEd25519 as SignatureAlgorithm>::PublicKey),
    RsaPublicKey(<SshRsa as SignatureAlgorithm>::PublicKey),
    UnknownPublicKey(UnknownPublicKey),
}

impl Encode for PublicKey {
    fn size(&self) -> usize {
        match self {
            PublicKey::Ed25519PublicKey(k) => k.size(),
            PublicKey::RsaPublicKey(k) => Encode::size(&"ssh-rsa") + k.size(),
            PublicKey::UnknownPublicKey(k) => Encode::size(&k.algo) + k.key.len(),
        }
    }
    fn encode<E: Encoder>(&self,c: &mut E) {
        match self {
            PublicKey::Ed25519PublicKey(k) => {
                Encode::encode(k, c);
            },
            PublicKey::RsaPublicKey(k) => {
                Encode::encode(&"ssh-rsa", c);
                Encode::encode(k, c);
            },
            PublicKey::UnknownPublicKey(k) => {
                Encode::encode(&k.algo, c);
                c.push_bytes(&k.key.as_slice());
            }
        }
    }
}

impl <'a> DecodeRef<'a> for PublicKey {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        None.or_else(|| {
                let mut d_ = d.clone();
                let r = DecodeRef::decode(&mut d_).map(PublicKey::Ed25519PublicKey);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = DecodeRef::decode(&mut d_).map(PublicKey::RsaPublicKey);
                if r.is_some() { *d = d_ };
                r
            })
            //.or_else(|| {
            //    let mut d_ = d.clone();
            //    let r = Decode::decode(&mut d_).map(|x| panic!(""));
            //    if r.is_some() { *d = d_ };
            //    r
            //})
    }
}

// TODO: Should be removable..
#[derive(Clone, Debug)]
pub enum Signature {
    // TODO SshEd..
    Ed25519Signature(<SshEd25519 as SignatureAlgorithm>::Signature),
    UnknownSignature(UnknownSignature),
}

impl Encode for Signature {
    fn size(&self) -> usize {
        match self {
            Signature::Ed25519Signature(k) => k.size(),
            Signature::UnknownSignature(k) => Encode::size(&k.algo) + k.signature.len(),
        }
    }
    fn encode<E: Encoder>(&self,c: &mut E) {
        match self {
            Signature::Ed25519Signature(k) => {
                Encode::encode(k, c);
            },
            Signature::UnknownSignature(k) => {
                Encode::encode(&k.algo, c);
                c.push_bytes(&k.signature.as_slice());
            }
        }
    }
}

impl <'a> DecodeRef<'a> for Signature {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        // TODO
        Some(Signature::Ed25519Signature(DecodeRef::decode(d)?))
    }
}
