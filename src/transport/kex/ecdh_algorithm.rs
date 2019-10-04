use crate::codec::*;
use rand_os::OsRng;

pub trait EcdhAlgorithm {
    type PublicKey: Copy;
    type EphemeralSecret;
    type SharedSecret;

    fn new() -> Self::EphemeralSecret;
    fn public(s: &Self::EphemeralSecret) -> Self::PublicKey;
    fn diffie_hellman(s: Self::EphemeralSecret, p: &Self::PublicKey) -> Self::SharedSecret;

    fn public_as_ref(x: &Self::PublicKey) -> &[u8];
    fn secret_as_ref(x: &Self::SharedSecret) -> &[u8];
}

#[derive(Debug)]
pub struct X25519 {}

impl EcdhAlgorithm for X25519 {
    type PublicKey = x25519_dalek::PublicKey;
    type EphemeralSecret = x25519_dalek::EphemeralSecret;
    type SharedSecret = x25519_dalek::SharedSecret;

    fn new() -> Self::EphemeralSecret {
        let mut csprng: OsRng = OsRng::new().unwrap();
        x25519_dalek::EphemeralSecret::new(&mut csprng)
    }

    fn public(s: &Self::EphemeralSecret) -> Self::PublicKey {
        x25519_dalek::PublicKey::from(s)
    }

    fn diffie_hellman(s: Self::EphemeralSecret, p: &Self::PublicKey) -> Self::SharedSecret {
        s.diffie_hellman(p)
    }

    fn public_as_ref(x: &Self::PublicKey) -> &[u8] {
        x.as_bytes().as_ref()
    }

    fn secret_as_ref(x: &Self::SharedSecret) -> &[u8] {
        x.as_bytes().as_ref()
    }
}

impl Encode for x25519_dalek::PublicKey {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + 32
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(32);
        e.push_bytes(self.as_bytes());
    }
}

impl Decode for x25519_dalek::PublicKey {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u32be(32)?;
        let mut buf: [u8;32] = [0;32];
        d.take_into(&mut buf)?;
        x25519_dalek::PublicKey::from(buf).into()
    }
}

