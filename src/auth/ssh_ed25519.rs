use super::*;
use crate::util::codec::*;
use std::any::Any;

#[derive(Debug)]
pub struct SshEd25519 {}

impl SshEd25519 {
    pub const NAME: &'static str = "ssh-ed25519";
}

#[derive(PartialEq, Clone, Debug)]
pub struct Ed25519PublicKey(pub [u8; 32]);

impl PublicKey for Ed25519PublicKey {
    fn algorithm(&self) -> &str {
        SshEd25519::NAME
    }

    fn verify_signature(&self, signature: &Signature, data: &[u8]) -> bool {
        use ed25519_dalek::{PublicKey, Signature};
        if signature.algorithm == SshEd25519::NAME {
            if signature.signature.len() == 64 {
                let mut sig: [u8; 64] = [0u8; 64];
                sig.copy_from_slice(&signature.signature[..64]);
                let sig = Signature::new(sig);
                if let Ok(key) = PublicKey::from_bytes(&self.0[..]) {
                    return key.verify_strict(data, &sig).is_ok();
                }
            }
        }
        false
    }

    fn equals(&self, public_key: &Box<dyn PublicKey>) -> bool {
        match public_key.as_any().downcast_ref::<Self>() {
            Some(other) => self == other,
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Encode for Ed25519PublicKey {
    fn size(&self) -> usize {
        (4 + 11 + 4 + 32) as usize
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(11)?;
        e.push_bytes(&SshEd25519::NAME)?;
        e.push_u32be(32)?;
        e.push_bytes(&self.0)
    }
}

impl Decode for Ed25519PublicKey {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let mut k = [0; 32];
        c.expect_u32be(11)?;
        c.expect_bytes(&SshEd25519::NAME)?;
        c.expect_u32be(32)?;
        c.take_into(&mut k)?;
        Some(Ed25519PublicKey(k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_ed25519_debug_01() {
        assert_eq!(format!("{:?}", SshEd25519 {}), "SshEd25519");
    }

    #[test]
    fn test_ssh_ed25519_publickey_debug_01() {
        assert_eq!(format!("{:?}", Ed25519PublicKey([3;32])), "Ed25519PublicKey([3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3])");
    }
}
