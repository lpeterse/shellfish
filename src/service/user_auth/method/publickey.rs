use super::*;
use crate::algorithm::*;

#[derive(Debug)]
pub struct PublicKeyMethod<S: AuthenticationAlgorithm> {
    pub public_key: S::Identity,
    pub signature: Option<S::Signature>,
}

impl<'a, S: AuthenticationAlgorithm> AuthMethod for PublicKeyMethod<S> {
    const NAME: &'static str = "publickey";
}

impl<S: AuthenticationAlgorithm> Encode for PublicKeyMethod<S>
where
    S::Identity: Encode,
    S::Signature: Encode,
{
    fn size(&self) -> usize {
        1 + Encode::size(&S::NAME)
            + Encode::size(&self.public_key)
            + match self.signature {
                None => 0,
                Some(ref x) => Encode::size(x),
            }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(self.signature.is_some() as u8);
        Encode::encode(&S::NAME, e);
        Encode::encode(&self.public_key, e);
        match self.signature {
            None => (),
            Some(ref x) => Encode::encode(x, e),
        }
    }
}

impl<'a, S: AuthenticationAlgorithm> DecodeRef<'a> for PublicKeyMethod<S>
where
    S::Identity: DecodeRef<'a>,
    S::Signature: DecodeRef<'a>,
{
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let b = d.take_u8()? != 0;
        let _: &str = DecodeRef::decode(d).filter(|x| *x == S::NAME)?;
        let public_key = DecodeRef::decode(d)?;
        let signature = if b { Some(DecodeRef::decode(d)?) } else { None };
        PublicKeyMethod {
            public_key,
            signature,
        }
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::algorithm::authentication::*;

    #[test]
    fn test_debug_01() {
        let pk = SshEd25519PublicKey([2; 32]);
        let sg = SshEd25519Signature([3; 64]);
        let x: PublicKeyMethod<SshEd25519> = PublicKeyMethod {
            public_key: pk,
            signature: Some(sg),
        };
        assert_eq!(format!("{:?}", x), "PublicKeyMethod { public_key: SshEd25519PublicKey([2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]), signature: Some(SshEd25519Signature([3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3])) }");
    }

    #[test]
    fn test_encode_01() {
        let pk = SshEd25519PublicKey([2; 32]);
        let x: PublicKeyMethod<SshEd25519> = PublicKeyMethod {
            public_key: pk,
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
            public_key: pk,
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
        assert_eq!(x.public_key.0, pk.0);
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
        assert_eq!(x.public_key.0, pk.0);
        assert_eq!(x.signature.unwrap().0[..], sg.0[..]);
    }
}
