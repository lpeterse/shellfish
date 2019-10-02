use crate::codec::*;
use super::*;

pub struct SshEd25519 {}

impl SshEd25519 {
    const NAME: &'static str = "ssh-ed25519";
    const NAME_SIZE: usize = 11;
    const PKEY_SIZE: usize = 32;
    const SKEY_SIZE: usize = 32;
    const SIG_SIZE: usize = 64;
}

impl SignatureAlgorithm for SshEd25519 {
    type PublicKey = SshEd25519PublicKey;
    type PrivateKey = SshEd25519PrivateKey;
    type Signature = SshEd25519Signature;
    type SignatureFlags = Ed25519SignatureFlags;

    const NAME: &'static str = SshEd25519::NAME;
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshEd25519PublicKey (pub [u8;32]);

impl Encode for SshEd25519PublicKey {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE) as u32);
        Encode::encode(&<SshEd25519 as SignatureAlgorithm>::NAME, e);
        Encode::encode(&self.0.as_ref(), e);
    }
}

impl Decode for SshEd25519PublicKey {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE))?;
        let _: &str = DecodeRef::decode(c).filter(|x| *x == <SshEd25519 as SignatureAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == 32)?;
        let mut k = [0;32];
        c.take_into(&mut k)?;
        Some(SshEd25519PublicKey(k))
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshEd25519PrivateKey (pub [u8;32]);

impl Encode for SshEd25519PrivateKey {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SKEY_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SKEY_SIZE) as u32);
        Encode::encode(&<SshEd25519 as SignatureAlgorithm>::NAME, e);
        Encode::encode(&self.0.as_ref(), e);
    }
}

impl Decode for SshEd25519PrivateKey {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SKEY_SIZE))?;
        let _: &str = DecodeRef::decode(c).filter(|x| *x == <SshEd25519 as SignatureAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == 32)?;
        let mut k = [0;32];
        c.take_into(&mut k)?;
        Some(SshEd25519PrivateKey(k))
    }
}

pub struct SshEd25519Signature ([u8;64]);

impl Clone for SshEd25519Signature {
    fn clone(&self) -> Self {
        Self (self.0)
    }
}

impl std::fmt::Debug for SshEd25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature {:?}..", &self.0[..32])
    }
}

impl Encode for SshEd25519Signature {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE) as u32);
        Encode::encode(&<SshEd25519 as SignatureAlgorithm>::NAME, e);
        Encode::encode(&self.0.as_ref(), e);
    }
}

impl Decode for SshEd25519Signature {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE))?;
        let _: &str = DecodeRef::decode(c).filter(|x| *x == <SshEd25519 as SignatureAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == SshEd25519::SIG_SIZE)?;
        let mut k = [0;64];
        c.take_into(&mut k)?;
        Some(SshEd25519Signature(k))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Ed25519SignatureFlags {}

impl Default for Ed25519SignatureFlags {
    fn default() -> Self {
        Self {}
    }
}

impl Into<u32> for Ed25519SignatureFlags {
    fn into(self) -> u32 {
        0
    }
}
