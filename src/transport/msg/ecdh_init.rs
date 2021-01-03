use super::super::ecdh::EcdhAlgorithm;
use super::Message;
use crate::util::codec::*;

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

impl<A: EcdhAlgorithm> SshEncode for MsgKexEcdhInit<A>
where
    A::PublicKey: SshEncode,
{
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push(&self.dh_public)
    }
}

impl<A: EcdhAlgorithm> SshDecode for MsgKexEcdhInit<A>
where
    A::PublicKey: SshDecode,
{
    fn decode<'a, D: SshDecoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            dh_public: SshDecodeRef::decode(c)?,
        })
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
        assert_eq!(&[30][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let msg = MsgKexEcdhInit::<()> { dh_public: () };
        assert_eq!(&Some(msg), &SshCodec::decode(&[30][..]));
    }
}
