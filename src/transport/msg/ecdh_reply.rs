use super::Message;
use crate::identity::*;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgKexEcdhReply {
    pub host_key: Identity,
    pub dh_public: Vec<u8>,
    pub signature: Signature,
}

impl MsgKexEcdhReply {
    pub fn new(host_key: Identity, dh_public: Vec<u8>, signature: Signature) -> Self {
        Self {
            host_key,
            dh_public,
            signature,
        }
    }
}

impl Message for MsgKexEcdhReply {
    const NUMBER: u8 = 31;
}

impl SshEncode for MsgKexEcdhReply {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push(&self.host_key)?;
        e.push_bytes_framed(&self.dh_public)?;
        e.push(&self.signature)
    }
}

impl SshDecode for MsgKexEcdhReply {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            host_key: d.take()?,
            dh_public: d.take_bytes_framed()?.into(),
            signature: d.take()?,
        }
        .into()
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ssh_ed25519::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestAlgorithm {}

    impl EcdhAlgorithm for TestAlgorithm {
        type PublicKey = String;
        type EphemeralSecret = String;
        type SharedSecret = String;

        fn new() -> Self::EphemeralSecret {
            "EPHEMERAL_SEConnectionRequestET".into()
        }
        fn public(_: &Self::EphemeralSecret) -> Self::PublicKey {
            "EPHEMERAL_PUBLIC".into()
        }
        fn diffie_hellman(_: Self::EphemeralSecret, _: &Self::PublicKey) -> Self::SharedSecret {
            "SHARED_SEConnectionRequestET".into()
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
        let host_key: Identity =
            Identity::Ed25519PublicKey(Ed25519PublicKey([23; 32]));
        let host_signature: Signature = Signature { algorithm: "ssh-ed25519".into(), signature: vec![47; 64] };
        let msg = MsgKexEcdhReply::<TestAlgorithm> {
            host_key,
            dh_public: ep,
            signature: host_signature,
        };

        let actual = SshCodec::encode(&msg);
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
        let host_key: Identity =
            Identity::Ed25519PublicKey(Ed25519PublicKey([23; 32]));
        let host_signature: Signature = Signature { algorithm: "ssh-ed25519".into(), signature: vec![47; 64] };

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

        let actual: MsgKexEcdhReply<TestAlgorithm> = SshCodec::decode(&input[..]).unwrap();
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
*/
