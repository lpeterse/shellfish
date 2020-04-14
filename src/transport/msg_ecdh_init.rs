use super::*;
use crate::message::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgKexEcdhInit<A: EcdhAlgorithm> {
    pub dh_public: A::PublicKey,
}

impl<A: EcdhAlgorithm> MsgKexEcdhInit<A> {
    pub fn new(dh_public: A::PublicKey) -> Self {
        Self { dh_public }
    }
}

impl<A: EcdhAlgorithm> Message for MsgKexEcdhInit<A> {
    const NUMBER: u8 = 30;
}

impl<A: EcdhAlgorithm> Encode for MsgKexEcdhInit<A>
where
    A::PublicKey: Encode,
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.dh_public)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.dh_public, c);
    }
}

impl<A: EcdhAlgorithm> Decode for MsgKexEcdhInit<A>
where
    A::PublicKey: Decode,
{
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            dh_public: DecodeRef::decode(c)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl EcdhAlgorithm for () {
        type PublicKey = ();
        type EphemeralSecret = ();
        type SharedSecret = ();

        fn new() -> Self::EphemeralSecret {
            ()
        }
        fn public(_: &Self::EphemeralSecret) -> Self::PublicKey {
            ()
        }
        fn diffie_hellman(_: Self::EphemeralSecret, _: &Self::PublicKey) -> Self::SharedSecret {
            ()
        }
        fn public_as_ref(_: &Self::PublicKey) -> &[u8] {
            &[]
        }
        fn secret_as_ref(_: &Self::SharedSecret) -> &[u8] {
            &[]
        }
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgKexEcdhInit::<()> { dh_public: () };
        assert_eq!(&[30][..], &BEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let msg = MsgKexEcdhInit::<()> { dh_public: () };
        assert_eq!(&Some(msg), &BDecoder::decode(&[30][..]));
    }
}
