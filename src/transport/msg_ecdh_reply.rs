use super::ecdh_algorithm::*;
use super::*;
use crate::algorithm::auth::*;
use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgKexEcdhReply<A: EcdhAlgorithm> {
    pub host_key: Identity,
    pub dh_public: A::PublicKey,
    pub signature: HostSignature,
}

impl<A: EcdhAlgorithm> Message for MsgKexEcdhReply<A> {
    const NUMBER: u8 = 31;
}

impl<A> Encode for MsgKexEcdhReply<A>
where
    A: EcdhAlgorithm,
    A::PublicKey: Encode,
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
            + self.host_key.size()
            + self.dh_public.size()
            + self.signature.size()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.host_key, e);
        Encode::encode(&self.dh_public, e);
        Encode::encode(&self.signature, e);
    }
}

impl<A: EcdhAlgorithm> Decode for MsgKexEcdhReply<A>
where
    A: EcdhAlgorithm,
    A::PublicKey: Decode,
{
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            host_key: DecodeRef::decode(d)?,
            dh_public: DecodeRef::decode(d)?,
            signature: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestAlgorithm {}

    impl EcdhAlgorithm for TestAlgorithm {
        type PublicKey = String;
        type EphemeralSecret = String;
        type SharedSecret = String;

        fn new() -> Self::EphemeralSecret {
            "EPHEMERAL_SECRET".into()
        }
        fn public(_: &Self::EphemeralSecret) -> Self::PublicKey {
            "EPHEMERAL_PUBLIC".into()
        }
        fn diffie_hellman(_: Self::EphemeralSecret, _: &Self::PublicKey) -> Self::SharedSecret {
            "SHARED_SECRET".into()
        }
        fn public_as_ref(pk: &Self::PublicKey) -> &[u8] {
            pk.as_ref()
        }
        fn secret_as_ref(sk: &Self::SharedSecret) -> &[u8] {
            sk.as_ref()
        }
    }

    #[test]
    fn test_encode_01() {
        let es = TestAlgorithm::new();
        let ep = TestAlgorithm::public(&es);
        let host_key: Identity = Identity::Ed25519Key(SshEd25519PublicKey([23; 32]));
        let host_signature: HostSignature =
            HostSignature::Ed25519Signature(SshEd25519Signature([47; 64]));
        let msg = MsgKexEcdhReply::<TestAlgorithm> {
            host_key,
            dh_public: ep,
            signature: host_signature,
        };

        let actual = BEncoder::encode(&msg);
        let expected: [u8; 163] = [
            31, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0,
            32, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
            23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 0, 0, 0, 16, 69, 80, 72, 69, 77, 69, 82,
            65, 76, 95, 80, 85, 66, 76, 73, 67, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101,
            100, 50, 53, 53, 49, 57, 0, 0, 0, 64, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
            47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
            47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
            47, 47, 47, 47, 47, 47, 47, 47,
        ];

        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_decode_01() {
        let es = TestAlgorithm::new();
        let ep = TestAlgorithm::public(&es);
        let host_key: Identity = Identity::Ed25519Key(SshEd25519PublicKey([23; 32]));
        let host_signature: HostSignature =
            HostSignature::Ed25519Signature(SshEd25519Signature([47; 64]));

        let input: [u8; 163] = [
            31, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0,
            32, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
            23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 0, 0, 0, 16, 69, 80, 72, 69, 77, 69, 82,
            65, 76, 95, 80, 85, 66, 76, 73, 67, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101,
            100, 50, 53, 53, 49, 57, 0, 0, 0, 64, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
            47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
            47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
            47, 47, 47, 47, 47, 47, 47, 47,
        ];

        let actual: MsgKexEcdhReply::<TestAlgorithm> = BDecoder::decode(&input[..]).unwrap();
        let expected = MsgKexEcdhReply::<TestAlgorithm> {
            host_key,
            dh_public: ep,
            signature: host_signature,
        };

        assert_eq!(actual.host_key, expected.host_key);
        assert_eq!(actual.dh_public, expected.dh_public);
        assert_eq!(actual.signature, expected.signature);
    }
}
