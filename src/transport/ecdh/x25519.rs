use super::*;
use rand_core::OsRng;
use std::convert::TryInto;

#[derive(Debug)]
pub struct X25519;

impl EcdhAlgorithm for X25519 {
    type PublicKey = x25519_dalek::PublicKey;
    type EphemeralSecret = x25519_dalek::EphemeralSecret;

    fn public_from_secret(s: &Self::EphemeralSecret) -> Self::PublicKey {
        x25519_dalek::PublicKey::from(s)
    }

    fn public_from_bytes(bytes: &[u8]) -> Option<Self::PublicKey> {
        let bytes: [u8; 32] = bytes.try_into().ok()?;
        Some(x25519_dalek::PublicKey::from(bytes))
    }

    fn public_as_bytes(x: &Self::PublicKey) -> &[u8] {
        x.as_bytes().as_ref()
    }

    fn secret_new() -> Self::EphemeralSecret {
        x25519_dalek::EphemeralSecret::new(&mut OsRng)
    }

    fn diffie_hellman(s: Self::EphemeralSecret, p: &Self::PublicKey) -> Secret {
        Secret::new(s.diffie_hellman(p).as_bytes())
    }
}
