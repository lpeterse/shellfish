mod hash;
mod x25519;

pub use self::hash::*;
pub use self::x25519::*;

pub trait EcdhAlgorithm {
    type PublicKey;
    type EphemeralSecret;
    type SharedSecret;

    fn new() -> Self::EphemeralSecret;
    fn public(s: &Self::EphemeralSecret) -> Self::PublicKey;
    fn diffie_hellman(s: Self::EphemeralSecret, p: &Self::PublicKey) -> Self::SharedSecret;

    fn public_as_ref(x: &Self::PublicKey) -> &[u8];
    fn secret_as_ref(x: &Self::SharedSecret) -> &[u8];
}
