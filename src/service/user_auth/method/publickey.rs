use super::*;
use crate::algorithm::*;

#[derive(Debug)]
pub struct PublicKeyMethod<S: AuthAlgorithm> {
    pub identity: S::AuthIdentity,
    pub signature: Option<S::AuthSignature>,
}

impl<'a, S: AuthAlgorithm> AuthMethod for PublicKeyMethod<S> {
    const NAME: &'static str = "publickey";
}

impl<S: AuthAlgorithm> Encode for PublicKeyMethod<S>
where
    S::AuthIdentity: Encode,
    S::AuthSignature: Encode,
{
    fn size(&self) -> usize {
        1 + Encode::size(&S::NAME)
            + Encode::size(&self.identity)
            + match self.signature {
                None => 0,
                Some(ref x) => Encode::size(x),
            }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(self.signature.is_some() as u8);
        Encode::encode(&S::NAME, e);
        Encode::encode(&self.identity, e);
        match self.signature {
            None => (),
            Some(ref x) => Encode::encode(x, e),
        }
    }
}

impl<'a, S: AuthAlgorithm> DecodeRef<'a> for PublicKeyMethod<S>
where
    S::AuthIdentity: DecodeRef<'a>,
    S::AuthSignature: DecodeRef<'a>,
{
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let b = d.take_u8()? != 0;
        let _: &str = DecodeRef::decode(d).filter(|x| *x == S::NAME)?;
        let identity = d.isolate_u32be(|x| DecodeRef::decode(x))?;
        let signature = if b { Some(DecodeRef::decode(d)?) } else { None };
        PublicKeyMethod {
            identity,
            signature,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::auth::*;

    #[test]
    fn test_debug_01() {
        let pk = SshEd25519PublicKey([2; 32]);
        let sg = SshEd25519Signature([3; 64]);
        let x: PublicKeyMethod<SshEd25519> = PublicKeyMethod {
            identity: pk,
            signature: Some(sg),
        };
        assert_eq!(format!("{:?}", x), "PublicKeyMethod { identity: SshEd25519PublicKey([2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]), signature: Some(SshEd25519Signature([3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3])) }");
    }

    #[test]
    fn test_encode_01() {
        let pk = SshEd25519PublicKey([2; 32]);
        let x: PublicKeyMethod<SshEd25519> = PublicKeyMethod {
            identity: pk,
            signature: None,
        };
        let actual = BEncoder::encode(&x);
        let expected = &[
            0, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0, 0,
            11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ][..];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_encode_02() {
        let pk = SshEd25519PublicKey([2; 32]);
        let sg = SshEd25519Signature([3; 64]);
        let x: PublicKeyMethod<SshEd25519> = PublicKeyMethod {
            identity: pk,
            signature: Some(sg),
        };
        let actual = BEncoder::encode(&x);
        let expected = &[
            1, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0, 0,
            11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 83,
            0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 64, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3,
        ][..];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_01() {
        let pk = SshEd25519PublicKey([2; 32]);
        let x: PublicKeyMethod<SshEd25519> = BDecoder::decode(
            &[
                0, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0,
                0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2,
                2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            ][..],
        )
        .unwrap();
        assert_eq!(x.identity.0, pk.0);
        assert_eq!(x.signature, None);
    }

    #[test]
    fn test_decode_02() {
        let pk = SshEd25519PublicKey([2; 32]);
        let sg = SshEd25519Signature([3; 64]);
        let x: PublicKeyMethod<SshEd25519> = BDecoder::decode(
            &[
                1, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0,
                0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2,
                2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0,
                0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0,
                64, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            ][..],
        )
        .unwrap();
        assert_eq!(x.identity.0, pk.0);
        assert_eq!(x.signature.unwrap().0[..], sg.0[..]);
    }
}
