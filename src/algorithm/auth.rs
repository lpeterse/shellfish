mod certificate;
mod identity;
mod public_key;
mod signature;
mod ssh_ed25519;
mod ssh_ed25519_cert;
mod ssh_rsa;

pub use self::certificate::*;
pub use self::identity::*;
pub use self::public_key::*;
pub use self::signature::*;
pub use self::ssh_ed25519::*;
pub use self::ssh_ed25519_cert::*;
pub use self::ssh_rsa::*;

use crate::codec::*;

/// Any algorithm used for authentication shall implement this trait.
///
/// The algorithms are expected to have an associated identity format (e.g. pubkey or certificate)
/// and a signature format (see associated types).
pub trait AuthAlgorithm {
    type AuthIdentity;
    type AuthSignature;
    type AuthSignatureFlags: Copy + Default + Into<u32>;

    /// The identifying algorithm name (e.g. `ssh-ed25519`).
    const NAME: &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_ed25519_key() -> SshEd25519PublicKey {
        SshEd25519PublicKey([3; 32])
    }

    fn example_ed25519_cert() -> SshEd25519Certificate {
        SshEd25519Certificate {
            nonce: [1; 32],
            pk: example_ed25519_key(),
            serial: 1234,
            type_: 1,
            key_id: "KEY_ID".into(),
            valid_principals: vec!["VALID_PRINCIPALS".into(), "MORE".into()],
            valid_after: u64::min_value(),
            valid_before: u64::max_value(),
            critical_options: vec![("OPTION1".into(), "".into()), ("OPTION2".into(), "".into())],
            extensions: vec![("EXT1".into(), "".into()), ("EXT2".into(), "".into())],
            reserved: vec![],
            signature_key: example_ed25519_key(),
            signature: SshEd25519Signature([5; 64]),
        }
    }

    fn example_ed25519_signature() -> SshEd25519Signature {
        SshEd25519Signature([5; 64])
    }

    fn example_rsa_key() -> SshRsaPublicKey {
        SshRsaPublicKey {
            public_e: vec![1, 2, 3],
            public_n: vec![4, 5, 6, 7, 8],
        }
    }

    #[test]
    fn test_algorithm_01() {
        let key = Identity::PublicKey(PublicKey::Ed25519(example_ed25519_key()));
        assert_eq!(key.algorithm(), "ssh-ed25519");
    }

    #[test]
    fn test_algorithm_02() {
        let key = Identity::Certificate(Certificate::Ed25519(example_ed25519_cert()));
        assert_eq!(key.algorithm(), "ssh-ed25519-cert-v01@openssh.com");
    }

    #[test]
    fn test_algorithm_03() {
        let key = Identity::PublicKey(PublicKey::Rsa(example_rsa_key()));
        assert_eq!(key.algorithm(), "ssh-rsa");
    }

    #[test]
    fn test_algorithm_04() {
        let key = Identity::PublicKey(PublicKey::Other("unknown".into()));
        assert_eq!(key.algorithm(), "unknown");
    }

    #[test]
    fn test_encode_01() {
        let key = Identity::PublicKey(PublicKey::Ed25519(example_ed25519_key()));
        let actual = BEncoder::encode(&key);
        let expected = [
            0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_encode_02() {
        let key = Identity::Certificate(Certificate::Ed25519(example_ed25519_cert()));
        let actual = BEncoder::encode(&key);
        let expected = [
            0, 0, 1, 130, 0, 0, 0, 32, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 45, 99,
            101, 114, 116, 45, 118, 48, 49, 64, 111, 112, 101, 110, 115, 115, 104, 46, 99, 111,
            109, 0, 0, 0, 32, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 32, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 4, 210, 0, 0, 0, 1,
            0, 0, 0, 6, 75, 69, 89, 95, 73, 68, 0, 0, 0, 28, 0, 0, 0, 16, 86, 65, 76, 73, 68, 95,
            80, 82, 73, 78, 67, 73, 80, 65, 76, 83, 0, 0, 0, 4, 77, 79, 82, 69, 0, 0, 0, 0, 0, 0,
            0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 30, 0, 0, 0, 7, 79, 80, 84, 73,
            79, 78, 49, 0, 0, 0, 0, 0, 0, 0, 7, 79, 80, 84, 73, 79, 78, 50, 0, 0, 0, 0, 0, 0, 0,
            24, 0, 0, 0, 4, 69, 88, 84, 49, 0, 0, 0, 0, 0, 0, 0, 4, 69, 88, 84, 50, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0,
            0, 0, 32, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53,
            49, 57, 0, 0, 0, 64, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_encode_03() {
        let key = Identity::PublicKey(PublicKey::Rsa(example_rsa_key()));
        let actual = BEncoder::encode(&key);
        let expected = [
            0, 0, 0, 27, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3, 1, 2, 3, 0, 0, 0,
            5, 4, 5, 6, 7, 8,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_decode_01() {
        let expected = Identity::PublicKey(PublicKey::Ed25519(example_ed25519_key()));
        let actual: Identity = BDecoder::decode(
            &[
                0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0,
                32, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_02() {
        let expected = Identity::Certificate(Certificate::Ed25519(example_ed25519_cert()));
        let actual: Identity = BDecoder::decode(
            &[
                0, 0, 1, 130, 0, 0, 0, 32, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 45, 99,
                101, 114, 116, 45, 118, 48, 49, 64, 111, 112, 101, 110, 115, 115, 104, 46, 99, 111,
                109, 0, 0, 0, 32, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 32, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 4, 210,
                0, 0, 0, 1, 0, 0, 0, 6, 75, 69, 89, 95, 73, 68, 0, 0, 0, 28, 0, 0, 0, 16, 86, 65,
                76, 73, 68, 95, 80, 82, 73, 78, 67, 73, 80, 65, 76, 83, 0, 0, 0, 4, 77, 79, 82, 69,
                0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 30, 0, 0,
                0, 7, 79, 80, 84, 73, 79, 78, 49, 0, 0, 0, 0, 0, 0, 0, 7, 79, 80, 84, 73, 79, 78,
                50, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 4, 69, 88, 84, 49, 0, 0, 0, 0, 0, 0, 0, 4,
                69, 88, 84, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104,
                45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 83, 0, 0, 0,
                11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 64, 5, 5, 5, 5, 5, 5,
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                5, 5,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_03() {
        let expected = Identity::PublicKey(PublicKey::Rsa(example_rsa_key()));
        let actual: Identity = BDecoder::decode(
            &[
                0, 0, 0, 27, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3, 1, 2, 3, 0,
                0, 0, 5, 4, 5, 6, 7, 8,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_04() {
        let expected = Identity::PublicKey(PublicKey::Other("unknown".into()));
        let actual: Identity = BDecoder::decode(
            &[
                0, 0, 0, 19, 0, 0, 0, 7, 117, 110, 107, 110, 111, 119, 110, 0, 0, 0, 4, 100, 97,
                116, 97,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_signature_encode_01() {
        let sig = Signature::Ed25519(example_ed25519_signature());
        let actual = BEncoder::encode(&sig);
        let expected = [
            0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 64,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, 5, 5, 5,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_signature_decode_01() {
        let expected = Signature::Ed25519(example_ed25519_signature());
        let actual: Signature = BDecoder::decode(
            &[
                0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0,
                64, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_signature_verify_valid_01() {
        let pk = PublicKey::Ed25519(SshEd25519PublicKey([
            75, 51, 174, 250, 168, 148, 30, 47, 57, 178, 223, 0, 217, 160, 197, 192, 229, 244, 195,
            102, 205, 139, 167, 208, 134, 184, 170, 190, 192, 44, 177, 47,
        ]));
        let sig = Signature::Ed25519(SshEd25519Signature([
            218, 91, 229, 121, 129, 106, 140, 188, 38, 182, 150, 75, 211, 82, 149, 5, 148, 185, 91,
            129, 31, 63, 30, 137, 187, 234, 165, 246, 130, 222, 222, 145, 233, 157, 119, 106, 129,
            16, 4, 174, 11, 40, 119, 151, 24, 56, 192, 12, 112, 89, 70, 172, 163, 89, 183, 123,
            244, 106, 208, 68, 88, 123, 26, 8,
        ]));
        let data = [
            78, 0, 134, 150, 89, 178, 20, 41, 42, 222, 78, 127, 161, 158, 105, 59, 33, 37, 222,
            103, 4, 44, 156, 174, 112, 125, 167, 190, 71, 199, 166, 114,
        ];
        assert!(sig.verify(&pk, &data[..]).is_some());
    }

    #[test]
    fn test_signature_verify_invalid_01() {
        let pk = PublicKey::Ed25519(SshEd25519PublicKey([
            75, 51, 174, 250, 168, 148, 30, 47, 57, 178, 223, 0, 217, 160, 197, 192, 229, 244, 195,
            102, 205, 139, 167, 208, 134, 184, 170, 190, 192, 44, 177, 47,
        ]));
        let sig = Signature::Ed25519(SshEd25519Signature([
            218, 91, 229, 121, 129, 106, 140, 188, 38, 182, 150, 75, 211, 82, 149, 5, 148, 185, 91,
            129, 31, 63, 30, 137, 187, 234, 165, 246, 130, 222, 222, 145, 233, 157, 119, 106, 129,
            16, 4, 174, 11, 40, 119, 151, 24, 56, 192, 12, 112, 89, 70, 172, 163, 89, 183, 123,
            244, 106, 208, 68, 88, 123, 26, 8,
        ]));
        let data = [
            78, 0, 134, 150, 89, 178, 20, 41, 42, 222, 78, 127, 161, 158, 105, 59, 33, 37, 222,
            103, 4, 44, 156, 174, 112, 125, 167, 190, 71, 199, 166,
            115, // last byte different!
        ];
        assert!(sig.verify(&pk, &data[..]).is_none());
    }

    #[test]
    fn test_signature_verify_mismatch_01() {
        let pk = PublicKey::Other("unknown".into());
        let sig = Signature::Ed25519(SshEd25519Signature([
            218, 91, 229, 121, 129, 106, 140, 188, 38, 182, 150, 75, 211, 82, 149, 5, 148, 185, 91,
            129, 31, 63, 30, 137, 187, 234, 165, 246, 130, 222, 222, 145, 233, 157, 119, 106, 129,
            16, 4, 174, 11, 40, 119, 151, 24, 56, 192, 12, 112, 89, 70, 172, 163, 89, 183, 123,
            244, 106, 208, 68, 88, 123, 26, 8,
        ]));
        let data = [];
        assert!(sig.verify(&pk, &data[..]).is_none());
    }
}
