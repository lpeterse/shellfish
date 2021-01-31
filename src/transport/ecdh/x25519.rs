use super::*;
use crate::util::codec::*;
use rand_core::OsRng;

#[derive(Debug)]
pub struct X25519;

impl EcdhAlgorithm for X25519 {
    type PublicKey = x25519_dalek::PublicKey;
    type EphemeralSecret = x25519_dalek::EphemeralSecret;
    type SharedSecret = x25519_dalek::SharedSecret;

    fn new() -> Self::EphemeralSecret {
        x25519_dalek::EphemeralSecret::new(&mut OsRng)
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

impl SshEncode for x25519_dalek::PublicKey {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(32)?;
        e.push_bytes(self.as_bytes())
    }
}

impl SshDecode for x25519_dalek::PublicKey {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u32be(32)?;
        let mut buf: [u8; 32] = [0; 32];
        d.take_bytes_into(&mut buf)?;
        x25519_dalek::PublicKey::from(buf).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x25519_encode_decode() {
        let s = X25519::new();
        let p1: <X25519 as EcdhAlgorithm>::PublicKey = <X25519 as EcdhAlgorithm>::public(&s);
        let v = SshCodec::encode(&p1).unwrap();
        let p2: <X25519 as EcdhAlgorithm>::PublicKey = SshCodec::decode(&v[..]).unwrap();
        assert_eq!(p1.as_bytes(), p2.as_bytes());
    }

    #[test]
    fn x25519_diffie_hellman() {
        let s1 = X25519::new();
        let s2 = X25519::new();
        let p1: <X25519 as EcdhAlgorithm>::PublicKey = <X25519 as EcdhAlgorithm>::public(&s1);
        let p2: <X25519 as EcdhAlgorithm>::PublicKey = <X25519 as EcdhAlgorithm>::public(&s2);
        let x1 = <X25519 as EcdhAlgorithm>::diffie_hellman(s1, &p2);
        let x2 = <X25519 as EcdhAlgorithm>::diffie_hellman(s2, &p1);
        assert_eq!(
            <X25519 as EcdhAlgorithm>::secret_as_ref(&x1),
            <X25519 as EcdhAlgorithm>::secret_as_ref(&x2)
        );
    }
}
