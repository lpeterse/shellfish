use super::*;
use crate::codec::*;

#[derive(Debug)]
pub struct SshEd25519 {}

impl SshEd25519 {
    const NAME: &'static str = "ssh-ed25519";
    const NAME_SIZE: usize = 11;
    const PKEY_SIZE: usize = 32;
    const SIG_SIZE: usize = 64;
}

impl AuthenticationAlgorithm for SshEd25519 {
    type Identity = SshEd25519PublicKey;
    type Signature = SshEd25519Signature;
    type SignatureFlags = SshEd25519SignatureFlags;

    const NAME: &'static str = SshEd25519::NAME;
}

#[derive(PartialEq, Clone, Debug)]
pub struct SshEd25519PublicKey(pub [u8; 32]);

impl Encode for SshEd25519PublicKey {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE) as u32);
        Encode::encode(&<SshEd25519 as AuthenticationAlgorithm>::NAME, e);
        Encode::encode(self.0.as_ref(), e);
    }
}

impl Decode for SshEd25519PublicKey {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be()
            .filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::PKEY_SIZE))?;
        let _: &str =
            DecodeRef::decode(c).filter(|x| *x == <SshEd25519 as AuthenticationAlgorithm>::NAME)?;
        c.take_u32be().filter(|x| *x as usize == 32)?;
        let mut k = [0; 32];
        c.take_into(&mut k)?;
        Some(SshEd25519PublicKey(k))
    }
}

pub struct SshEd25519Signature(pub [u8; 64]);

impl PartialEq for SshEd25519Signature {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}

impl Clone for SshEd25519Signature {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl std::fmt::Debug for SshEd25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature({:?})", &self.0[..])
    }
}

impl Encode for SshEd25519Signature {
    fn size(&self) -> usize {
        4 + 4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE) as u32);
        Encode::encode(&<SshEd25519 as AuthenticationAlgorithm>::NAME, e);
        Encode::encode(self.0.as_ref(), e);
    }
}

impl Decode for SshEd25519Signature {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be()
            .filter(|x| *x as usize == (4 + SshEd25519::NAME_SIZE + 4 + SshEd25519::SIG_SIZE))?;
        let _: &str =
            DecodeRef::decode(c).filter(|x| *x == <SshEd25519 as AuthenticationAlgorithm>::NAME)?;
        c.expect_u32be(SshEd25519::SIG_SIZE as u32)?;
        let mut k = [0; 64];
        c.take_into(&mut k)?;
        Some(SshEd25519Signature(k))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SshEd25519SignatureFlags {}

impl Default for SshEd25519SignatureFlags {
    fn default() -> Self {
        Self {}
    }
}

impl Into<u32> for SshEd25519SignatureFlags {
    fn into(self) -> u32 {
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ssh_ed25519_debug_01() {
        assert_eq!(format!("{:?}", SshEd25519 {}), "SshEd25519");
    }

    #[test]
    fn test_ssh_ed25519_publickey_debug_01() {
        assert_eq!(format!("{:?}", SshEd25519PublicKey([3;32])), "SshEd25519PublicKey([3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3])");
    }

    #[test]
    fn test_ssh_ed25519_signature_debug_01() {
        assert_eq!(format!("{:?}", SshEd25519Signature([3;64])), "Ed25519Signature([3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3])");
    }

    #[test]
    fn test_ssh_ed25519_flags_debug_01() {
        assert_eq!(
            format!("{:?}", SshEd25519SignatureFlags {}),
            "SshEd25519SignatureFlags"
        );
    }

    #[test]
    fn test_ssh_ed25519_signature_clone_01() {
        let x = SshEd25519Signature([3; 64]);
        let y = x.clone();
        assert_eq!(&x.0[..], &y.0[..]);
    }

    #[test]
    fn test_ssh_ed25519_flags_default_01() {
        match SshEd25519SignatureFlags::default() {
            SshEd25519SignatureFlags {} => (),
        }
    }

    #[test]
    fn test_ssh_ed25519_flags_into_u32_01() {
        let x = SshEd25519SignatureFlags {};
        let y: u32 = x.into();
        assert_eq!(y, 0);
    }
}
