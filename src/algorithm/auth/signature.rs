use super::*;

/// A user's or host's signature.
#[derive(Clone, Debug, PartialEq)]
pub enum HostSignature {
    Ed25519Signature(<SshEd25519 as AuthAlgorithm>::AuthSignature),
}

impl HostSignature {
    /// Verify a signature by given identity over given data.
    ///
    /// Returns error in case the algorithms do not match or the signature is invalid.
    pub fn verify(&self, id: &PublicKey, data: &[u8]) -> Result<(), SignatureError> {
        match (self, id) {
            (Self::Ed25519Signature(s), PublicKey::Ed25519(i)) => {
                use ed25519_dalek::{PublicKey, Signature};
                let key = PublicKey::from_bytes(&i.0[..])
                    .map_err(|_| SignatureError::InvalidSignature)?;
                let sig = Signature::from_bytes(&s.0[..])
                    .map_err(|_| SignatureError::InvalidSignature)?;
                key.verify(data, &sig)
                    .map_err(|_| SignatureError::InvalidSignature)
            }
            _ => Err(SignatureError::AlgorithmMismatch),
        }
    }
}
