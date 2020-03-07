use super::*;

/// A user's or host's signature.
#[derive(Clone, Debug, PartialEq)]
pub enum Signature {
    Ed25519(<SshEd25519 as AuthAlgorithm>::AuthSignature),
}

impl Signature {
    /// Verify a signature for given public key over given data.
    ///
    /// Returns error in case the algorithms do not match or the signature is invalid.
    pub fn verify(&self, id: &PublicKey, data: &[u8]) -> Option<()> {
        match (self, id) {
            (Self::Ed25519(s), PublicKey::Ed25519(i)) => {
                use ed25519_dalek::{PublicKey, Signature};
                let key = PublicKey::from_bytes(&i.0[..]).ok()?;
                let sig = Signature::from_bytes(&s.0[..]).ok()?;
                key.verify(data, &sig).ok()
            }
            _ => None
        }
    }
}

impl Encode for Signature {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519(k) => k.size(),
        }
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        match self {
            Self::Ed25519(k) => {
                Encode::encode(k, c);
            }
        }
    }
}

impl<'a> DecodeRef<'a> for Signature {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Some(Self::Ed25519(DecodeRef::decode(d)?))
    }
}
