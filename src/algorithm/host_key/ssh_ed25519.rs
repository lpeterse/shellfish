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
    type PublicKey = Ed25519PublicKey;
    type PrivateKey = Ed25519PrivateKey;
    type Signature = Ed25519Signature;

    const NAME: &'static str = SshEd25519::NAME;
}

#[derive(PartialEq, Clone, Debug)]
pub struct Ed25519PublicKey (pub [u8;32]);

impl Encode for Ed25519PublicKey {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE) as u32);
        Encode::encode(&<SshEd25519 as SignatureAlgorithm>::NAME, e);
        Encode::encode(&self.0.as_ref(), e);
    }
}

impl <'a> Decode<'a> for Ed25519PublicKey {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE))?;
        let _: &str = Decode::decode(c).filter(|x| *x == <SshEd25519 as SignatureAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == 32)?;
        let mut k = [0;32];
        c.take_into(&mut k)?;
        Some(Ed25519PublicKey(k))
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Ed25519PrivateKey (pub [u8;32]);

impl Encode for Ed25519PrivateKey {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SKEY_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SKEY_SIZE) as u32);
        Encode::encode(&<SshEd25519 as SignatureAlgorithm>::NAME, e);
        Encode::encode(&self.0.as_ref(), e);
    }
}

impl <'a> Decode<'a> for Ed25519PrivateKey {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SKEY_SIZE))?;
        let _: &str = Decode::decode(c).filter(|x| *x == <SshEd25519 as SignatureAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == 32)?;
        let mut k = [0;32];
        c.take_into(&mut k)?;
        Some(Ed25519PrivateKey(k))
    }
}

pub struct Ed25519Signature ([u8;64]);

impl Clone for Ed25519Signature {
    fn clone(&self) -> Self {
        Self (self.0)
    }
}

impl std::fmt::Debug for Ed25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature {:?}..", &self.0[..32])
    }
}

impl Encode for Ed25519Signature {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE) as u32);
        Encode::encode(&<SshEd25519 as SignatureAlgorithm>::NAME, e);
        Encode::encode(&self.0.as_ref(), e);
    }
}

impl <'a> Decode<'a> for Ed25519Signature {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE))?;
        let _: &str = Decode::decode(c).filter(|x| *x == <SshEd25519 as SignatureAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == SshEd25519::SIG_SIZE)?;
        let mut k = [0;64];
        c.take_into(&mut k)?;
        Some(Ed25519Signature(k))
    }
}

// SIGNATURE: [14, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 64, 125, 174, 167, 124, 213, 4, 10, 162, 185, 153, 4, 132, 247, 61, 193, 37, 149, 114, 49, 64, 215, 178, 164, 59, 248, 145, 0, 15, 76, 2, 67, 140, 238, 192, 49, 75, 73, 137, 131, 19, 185, 193, 53, 191, 180, 2, 188, 66, 38, 31, 130, 14, 30, 232, 29, 14, 135, 96, 11, 17, 54, 212, 240, 0]
