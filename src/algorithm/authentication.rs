mod ssh_ed25519;
mod ssh_ed25519_cert;
mod ssh_rsa;
mod unknown;

pub use self::ssh_ed25519::*;
pub use self::ssh_ed25519_cert::*;
pub use self::ssh_rsa::*;
pub use self::unknown::*;

use crate::codec::*;

/// Any algorithm used for authentication shall implement this trait.
///
/// The algorithms are expected to have an associated identity format (e.g. pubkey or certificate)
/// and a signature format (see associated types).
pub trait AuthenticationAlgorithm {
    type Identity;
    type Signature;
    type SignatureFlags: Copy + Default + Into<u32>;

    /// The identifying algorithm name (e.g. `ssh-ed25519`).
    const NAME: &'static str;
}

/// A user's or host's identity.
///
/// This is either just a key or a certificate.
/// An identity is `Unknown` if it is not implemented and cannot be interpreted by us.
#[derive(Clone, Debug, PartialEq)]
pub enum HostIdentity {
    Ed25519Cert(<SshEd25519Cert as AuthenticationAlgorithm>::Identity),
    Ed25519Key(<SshEd25519 as AuthenticationAlgorithm>::Identity),
    RsaKey(<SshRsa as AuthenticationAlgorithm>::Identity),
    Unknown(UnknownIdentity),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PublicKey {
    Ed25519(SshEd25519PublicKey), // FIXME: Add other algos
}

impl PublicKey {
    pub fn decode<'a, D: Decoder<'a>>(d: &mut D, algo: &str) -> Option<PublicKey> {
        match algo {
            <SshEd25519 as AuthenticationAlgorithm>::NAME => {
                Decode::decode(d).map(PublicKey::Ed25519)
            }
            _ => None,
        }
    }
}

impl HostIdentity {
    /// For a given identity, yield the corresponding algorithm name.
    ///
    /// NOTE: This is implies that the relation is a bijection which might turn out as a wrong
    /// assumption in the future. Feel free to fix this as necessary.
    pub fn algorithm(&self) -> &str {
        match self {
            Self::Ed25519Cert(_) => <SshEd25519Cert as AuthenticationAlgorithm>::NAME,
            Self::Ed25519Key(_) => <SshEd25519 as AuthenticationAlgorithm>::NAME,
            Self::RsaKey(_) => <SshRsa as AuthenticationAlgorithm>::NAME,
            Self::Unknown(x) => x.algo.as_str(),
        }
    }

    pub fn is_valid_cert(&self, cakey: &PublicKey) -> bool {
        match (self, cakey) {
            (Self::Ed25519Cert(_), PublicKey::Ed25519(_)) => false, // FIXME
            _ => false,
        }
    }

    pub fn is_pubkey(&self, pubkey: &PublicKey) -> bool {
        match (self, pubkey) {
            (Self::Ed25519Key(x), PublicKey::Ed25519(y)) => x == y,
            //(Self::RsaKey(x), Self::RsaKey(y)) => x == y, FIXME
            _ => false,
        }
    }
}

impl Encode for HostIdentity {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519Cert(x) => Encode::size(x),
            Self::Ed25519Key(x) => Encode::size(x),
            Self::RsaKey(x) => Encode::size(x),
            Self::Unknown(x) => Encode::size(x),
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::Ed25519Cert(x) => Encode::encode(x, e),
            Self::Ed25519Key(x) => Encode::encode(x, e),
            Self::RsaKey(x) => Encode::encode(x, e),
            Self::Unknown(x) => Encode::encode(x, e),
        }
    }
}

impl Decode for HostIdentity {
    // FIXME
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        None.or_else(|| {
            let mut d_ = d.clone();
            let r = d_.isolate_u32be(|x| DecodeRef::decode(x).map(Self::Ed25519Key));
            if r.is_some() {
                *d = d_;
                log::error!("HKJHKJH");
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Self::Ed25519Cert);
            if r.is_some() {
                *d = d_
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Self::RsaKey);
            if r.is_some() {
                *d = d_
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = Decode::decode(&mut d_).map(Self::Unknown);
            if r.is_some() {
                *d = d_
            };
            r
        })
    }
}

/// A user's or host's signature.
#[derive(Clone, Debug, PartialEq)]
pub enum HostSignature {
    Ed25519Signature(<SshEd25519 as AuthenticationAlgorithm>::Signature),
}

impl HostSignature {
    /// Verify a signature by given identity over given data.
    ///
    /// Returns error in case the algorithms do not match or the signature is invalid.
    pub fn verify(&self, id: &HostIdentity, data: &[u8]) -> Result<(), SignatureError> {
        match (self, id) {
            (Self::Ed25519Signature(s), HostIdentity::Ed25519Key(i)) => {
                use ed25519_dalek::{PublicKey, Signature};
                let key = PublicKey::from_bytes(&i.0[..])
                    .map_err(|_| SignatureError::InvalidSignature)?;
                let sig = Signature::from_bytes(&s.0[..])
                    .map_err(|_| SignatureError::InvalidSignature)?;
                key.verify(data, &sig)
                    .map_err(|_| SignatureError::InvalidSignature)
            }
            _ => Err(SignatureError::AlgorithmMismatch),
        }
    }
}

impl Encode for HostSignature {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519Signature(k) => k.size(),
        }
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        match self {
            Self::Ed25519Signature(k) => {
                Encode::encode(k, c);
            }
        }
    }
}

impl<'a> DecodeRef<'a> for HostSignature {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Some(Self::Ed25519Signature(DecodeRef::decode(d)?))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SignatureError {
    InvalidSignature,
    AlgorithmMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

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
            public_e: BigUint::new(vec![65537]),
            public_n: BigUint::new(vec![
                1536924887, 1797284974, 3382208288, 91659320, 2738779923, 2905806383, 784289269,
                2160251933, 2238530493, 590023733, 173153565, 1244184158, 1836004836, 501213006,
                3586944145, 4283753687, 1957666482, 1993487836, 4124955837, 2966072476, 922893823,
                4002256118, 3748178068, 4000305288, 2947583907, 2383413512, 3078464384, 3460784445,
                3051753273, 248334581, 3707075206, 1293062625, 2121611348, 2966337243, 2594920081,
                3642317279, 1596726420, 2299993577, 2017127361, 3648103130, 1911636912, 880490666,
                1906911972, 2635741097, 3463104586, 3515935238, 3546269984, 3718626165, 2704397764,
                2466372219, 3518795182, 1908152768, 3218742931, 302152331, 1517143872, 1165089994,
                3059788424, 86980362, 1256028572, 1213756191, 3579960868, 587034069, 3731024158,
                3268237117,
            ]),
        }
    }

    fn example_unknown_identity() -> UnknownIdentity {
        UnknownIdentity {
            algo: "unknown".into(),
            data: Vec::from(&b"data"[..]),
        }
    }

    #[test]
    fn test_algorithm_01() {
        let key = HostIdentity::Ed25519Key(example_ed25519_key());
        assert_eq!(key.algorithm(), "ssh-ed25519");
    }

    #[test]
    fn test_algorithm_02() {
        let key = HostIdentity::Ed25519Cert(example_ed25519_cert());
        assert_eq!(key.algorithm(), "ssh-ed25519-cert-v01@openssh.com");
    }

    #[test]
    fn test_algorithm_03() {
        let key = HostIdentity::RsaKey(example_rsa_key());
        assert_eq!(key.algorithm(), "ssh-rsa");
    }

    #[test]
    fn test_algorithm_04() {
        let key = HostIdentity::Unknown(example_unknown_identity());
        assert_eq!(key.algorithm(), "unknown");
    }

    #[test]
    fn test_encode_01() {
        let key = HostIdentity::Ed25519Key(example_ed25519_key());
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
        let key = HostIdentity::Ed25519Cert(example_ed25519_cert());
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
        let key = HostIdentity::RsaKey(example_rsa_key());
        let actual = BEncoder::encode(&key);
        let expected = [
            0, 0, 1, 23, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3, 1, 0, 1, 0, 0, 1,
            1, 0, 194, 205, 87, 61, 222, 98, 233, 30, 34, 253, 109, 213, 213, 97, 222, 36, 72, 88,
            115, 31, 74, 221, 121, 156, 5, 47, 55, 10, 182, 96, 170, 136, 69, 113, 220, 202, 90,
            109, 199, 64, 18, 2, 122, 139, 191, 218, 30, 147, 113, 188, 25, 192, 209, 188, 141,
            174, 147, 1, 218, 123, 161, 49, 213, 196, 221, 165, 187, 117, 211, 95, 201, 32, 209,
            144, 234, 6, 206, 106, 200, 74, 157, 26, 55, 169, 113, 169, 42, 228, 52, 123, 56, 170,
            113, 241, 67, 176, 217, 113, 162, 218, 120, 58, 235, 193, 137, 23, 29, 233, 95, 44, 28,
            148, 217, 25, 89, 223, 154, 171, 86, 145, 176, 206, 182, 219, 126, 117, 56, 84, 77, 18,
            145, 225, 220, 245, 122, 134, 14, 205, 72, 245, 181, 230, 15, 57, 206, 71, 97, 61, 183,
            125, 163, 128, 142, 16, 1, 8, 175, 176, 143, 163, 238, 111, 208, 136, 223, 104, 168,
            148, 238, 141, 148, 246, 55, 2, 61, 255, 176, 202, 172, 156, 245, 221, 212, 189, 118,
            210, 53, 220, 116, 175, 158, 178, 255, 84, 228, 215, 213, 204, 108, 145, 29, 223, 231,
            78, 109, 111, 53, 228, 74, 40, 190, 94, 10, 82, 29, 29, 35, 43, 12, 53, 133, 109, 67,
            189, 128, 194, 212, 29, 46, 191, 77, 245, 173, 51, 22, 47, 163, 62, 119, 19, 5, 118,
            156, 56, 201, 152, 103, 32, 107, 32, 100, 110, 91, 155, 156, 215,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_encode_04() {
        let key = HostIdentity::Unknown(example_unknown_identity());
        let actual = BEncoder::encode(&key);
        let expected = [
            0, 0, 0, 7, 117, 110, 107, 110, 111, 119, 110, 0, 0, 0, 4, 100, 97, 116, 97,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_decode_01() {
        let expected = HostIdentity::Ed25519Key(example_ed25519_key());
        let actual: HostIdentity = BDecoder::decode(
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
        let expected = HostIdentity::Ed25519Cert(example_ed25519_cert());
        let actual: HostIdentity = BDecoder::decode(
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
        let expected = HostIdentity::RsaKey(example_rsa_key());
        let actual: HostIdentity = BDecoder::decode(
            &[
                0, 0, 1, 23, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3, 1, 0, 1, 0,
                0, 1, 1, 0, 194, 205, 87, 61, 222, 98, 233, 30, 34, 253, 109, 213, 213, 97, 222,
                36, 72, 88, 115, 31, 74, 221, 121, 156, 5, 47, 55, 10, 182, 96, 170, 136, 69, 113,
                220, 202, 90, 109, 199, 64, 18, 2, 122, 139, 191, 218, 30, 147, 113, 188, 25, 192,
                209, 188, 141, 174, 147, 1, 218, 123, 161, 49, 213, 196, 221, 165, 187, 117, 211,
                95, 201, 32, 209, 144, 234, 6, 206, 106, 200, 74, 157, 26, 55, 169, 113, 169, 42,
                228, 52, 123, 56, 170, 113, 241, 67, 176, 217, 113, 162, 218, 120, 58, 235, 193,
                137, 23, 29, 233, 95, 44, 28, 148, 217, 25, 89, 223, 154, 171, 86, 145, 176, 206,
                182, 219, 126, 117, 56, 84, 77, 18, 145, 225, 220, 245, 122, 134, 14, 205, 72, 245,
                181, 230, 15, 57, 206, 71, 97, 61, 183, 125, 163, 128, 142, 16, 1, 8, 175, 176,
                143, 163, 238, 111, 208, 136, 223, 104, 168, 148, 238, 141, 148, 246, 55, 2, 61,
                255, 176, 202, 172, 156, 245, 221, 212, 189, 118, 210, 53, 220, 116, 175, 158, 178,
                255, 84, 228, 215, 213, 204, 108, 145, 29, 223, 231, 78, 109, 111, 53, 228, 74, 40,
                190, 94, 10, 82, 29, 29, 35, 43, 12, 53, 133, 109, 67, 189, 128, 194, 212, 29, 46,
                191, 77, 245, 173, 51, 22, 47, 163, 62, 119, 19, 5, 118, 156, 56, 201, 152, 103,
                32, 107, 32, 100, 110, 91, 155, 156, 215,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_04() {
        let expected = HostIdentity::Unknown(example_unknown_identity());
        let actual: HostIdentity = BDecoder::decode(
            &[
                0, 0, 0, 7, 117, 110, 107, 110, 111, 119, 110, 0, 0, 0, 4, 100, 97, 116, 97,
            ][..],
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_signature_encode_01() {
        let sig = HostSignature::Ed25519Signature(example_ed25519_signature());
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
        let expected = HostSignature::Ed25519Signature(example_ed25519_signature());
        let actual: HostSignature = BDecoder::decode(
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
        let id = HostIdentity::Ed25519Key(SshEd25519PublicKey([
            75, 51, 174, 250, 168, 148, 30, 47, 57, 178, 223, 0, 217, 160, 197, 192, 229, 244, 195,
            102, 205, 139, 167, 208, 134, 184, 170, 190, 192, 44, 177, 47,
        ]));
        let sig = HostSignature::Ed25519Signature(SshEd25519Signature([
            218, 91, 229, 121, 129, 106, 140, 188, 38, 182, 150, 75, 211, 82, 149, 5, 148, 185, 91,
            129, 31, 63, 30, 137, 187, 234, 165, 246, 130, 222, 222, 145, 233, 157, 119, 106, 129,
            16, 4, 174, 11, 40, 119, 151, 24, 56, 192, 12, 112, 89, 70, 172, 163, 89, 183, 123,
            244, 106, 208, 68, 88, 123, 26, 8,
        ]));
        let data = [
            78, 0, 134, 150, 89, 178, 20, 41, 42, 222, 78, 127, 161, 158, 105, 59, 33, 37, 222,
            103, 4, 44, 156, 174, 112, 125, 167, 190, 71, 199, 166, 114,
        ];
        assert!(sig.verify(&id, &data[..]).is_ok());
    }

    #[test]
    fn test_signature_verify_invalid_01() {
        let id = HostIdentity::Ed25519Key(SshEd25519PublicKey([
            75, 51, 174, 250, 168, 148, 30, 47, 57, 178, 223, 0, 217, 160, 197, 192, 229, 244, 195,
            102, 205, 139, 167, 208, 134, 184, 170, 190, 192, 44, 177, 47,
        ]));
        let sig = HostSignature::Ed25519Signature(SshEd25519Signature([
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
        match sig.verify(&id, &data[..]) {
            Err(SignatureError::InvalidSignature) => (),
            _ => panic!(),
        }
    }

    #[test]
    fn test_signature_verify_mismatch_01() {
        let id = HostIdentity::Unknown(UnknownIdentity {
            algo: "unknown".into(),
            data: vec![],
        });
        let sig = HostSignature::Ed25519Signature(SshEd25519Signature([
            218, 91, 229, 121, 129, 106, 140, 188, 38, 182, 150, 75, 211, 82, 149, 5, 148, 185, 91,
            129, 31, 63, 30, 137, 187, 234, 165, 246, 130, 222, 222, 145, 233, 157, 119, 106, 129,
            16, 4, 174, 11, 40, 119, 151, 24, 56, 192, 12, 112, 89, 70, 172, 163, 89, 183, 123,
            244, 106, 208, 68, 88, 123, 26, 8,
        ]));
        let data = [];
        match sig.verify(&id, &data[..]) {
            Err(SignatureError::AlgorithmMismatch) => (),
            _ => panic!(),
        }
    }
    #[test]
    fn test_signature_error_debug_01() {
        assert_eq!(
            format!("{:?}", SignatureError::InvalidSignature),
            "InvalidSignature"
        );
        assert_eq!(
            format!("{:?}", SignatureError::AlgorithmMismatch),
            "AlgorithmMismatch"
        );
    }
}
