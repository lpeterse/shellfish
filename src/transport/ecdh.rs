mod hash;
mod x25519;

pub use self::hash::*;
pub use self::x25519::*;
use super::secret::Secret;

pub trait EcdhAlgorithm {
    type PublicKey;
    type EphemeralSecret;

    fn public_from_secret(s: &Self::EphemeralSecret) -> Self::PublicKey;
    fn public_from_bytes(bytes: &[u8]) -> Option<Self::PublicKey>;
    fn public_as_bytes(x: &Self::PublicKey) -> &[u8];

    fn secret_new() -> Self::EphemeralSecret;

    fn diffie_hellman(s: Self::EphemeralSecret, p: &Self::PublicKey) -> Secret;
}
