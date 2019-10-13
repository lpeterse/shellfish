use super::ecdh_algorithm::*;
use super::msg_kex_init::MsgKexInit;
use crate::algorithm::authentication::*;
use crate::codec::*;
use crate::transport::identification::*;

use sha2::{Digest, Sha256};

pub struct KexEcdhHash<'a, A: EcdhAlgorithm> {
    pub client_identification: &'a Identification<&'static str>,
    pub server_identification: &'a Identification<String>,
    pub client_kex_init: &'a MsgKexInit,
    pub server_kex_init: &'a MsgKexInit,
    pub server_host_key: &'a HostIdentity,
    pub dh_client_key: &'a A::PublicKey,
    pub dh_server_key: &'a A::PublicKey,
    pub dh_secret: &'a [u8],
}

impl<'a, A: EcdhAlgorithm> KexEcdhHash<'a, A> {
    pub fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(Encode::size(self.client_identification) as u32);
        Encode::encode(self.client_identification, e);

        e.push_u32be(Encode::size(self.server_identification) as u32);
        Encode::encode(self.server_identification, e);

        e.push_u32be(Encode::size(self.client_kex_init) as u32);
        Encode::encode(self.client_kex_init, e);

        e.push_u32be(Encode::size(self.server_kex_init) as u32);
        Encode::encode(self.server_kex_init, e);

        Encode::encode(self.server_host_key, e);

        e.push_u32be(A::public_as_ref(self.dh_client_key).len() as u32);
        e.push_bytes(&A::public_as_ref(self.dh_client_key));

        e.push_u32be(A::public_as_ref(self.dh_server_key).len() as u32);
        e.push_bytes(&A::public_as_ref(self.dh_server_key));

        Encode::encode(&MPInt(self.dh_secret), e);
    }

    pub fn sha256(&self) -> [u8; 32] {
        let mut sha256 = Sha256::new();
        self.encode(&mut sha256);
        let mut digest = [0; 32];
        digest.copy_from_slice(sha256.result().as_slice());
        digest
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::algorithm::encryption::*;
    use crate::algorithm::kex::*;
    use crate::transport::kex::*;

    #[test]
    fn test_kex_hash_01() {
        let client_identification = Identification::new("hssh_0.1.0.0".into(), "".into());
        let server_identification = Identification::new("hssh_0.1.0.0".into(), "".into());
        let client_kex_init = MsgKexInit {
            cookie: KexCookie([
                146, 105, 253, 96, 98, 147, 65, 76, 222, 166, 168, 241, 53, 43, 45, 168,
            ]),
            kex_algorithms: vec![<Curve25519Sha256AtLibsshDotOrg as KexAlgorithm>::NAME.into()],
            server_host_key_algorithms: vec!["ssh-ed25519".into()],
            encryption_algorithms_client_to_server: vec![
                <Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME.into(),
            ],
            encryption_algorithms_server_to_client: vec![
                <Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME.into(),
            ],
            mac_algorithms_client_to_server: vec![],
            mac_algorithms_server_to_client: vec![],
            compression_algorithms_client_to_server: vec!["none".into()],
            compression_algorithms_server_to_client: vec!["none".into()],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        };
        let server_kex_init = MsgKexInit {
            cookie: KexCookie([
                120, 224, 145, 197, 172, 191, 2, 206, 157, 48, 249, 184, 200, 249, 43, 201,
            ]),
            kex_algorithms: vec![<Curve25519Sha256AtLibsshDotOrg as KexAlgorithm>::NAME.into()],
            server_host_key_algorithms: vec!["ssh-ed25519".into()],
            encryption_algorithms_client_to_server: vec![
                <Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME.into(),
            ],
            encryption_algorithms_server_to_client: vec![
                <Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME.into(),
            ],
            mac_algorithms_client_to_server: vec![],
            mac_algorithms_server_to_client: vec![],
            compression_algorithms_client_to_server: vec!["none".into()],
            compression_algorithms_server_to_client: vec!["none".into()],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        };
        let server_host_key = HostIdentity::Ed25519Key(SshEd25519PublicKey([
            106, 114, 105, 46, 246, 21, 248, 172, 243, 187, 200, 45, 247, 246, 225, 218, 206, 250,
            145, 15, 246, 140, 131, 40, 234, 255, 135, 177, 8, 161, 128, 79,
        ]));
        let dh_client_key = x25519_dalek::PublicKey::from([
            163, 184, 73, 53, 101, 235, 117, 249, 31, 97, 178, 63, 135, 35, 65, 5, 189, 180, 255,
            250, 242, 232, 76, 164, 186, 212, 21, 0, 223, 144, 162, 77,
        ]);
        let dh_server_key = x25519_dalek::PublicKey::from([
            236, 229, 149, 54, 50, 179, 149, 65, 53, 52, 47, 205, 191, 6, 241, 2, 134, 85, 228, 18,
            66, 201, 189, 121, 8, 17, 122, 81, 175, 192, 25, 58,
        ]);
        let dh_secret = [
            81, 115, 212, 227, 1, 156, 126, 179, 66, 238, 221, 162, 9, 2, 163, 168, 217, 121, 91,
            96, 227, 131, 212, 209, 11, 219, 182, 110, 136, 28, 151, 2,
        ];

        let sha256_digest = [
            189, 219, 42, 55, 209, 120, 44, 65, 77, 213, 114, 209, 26, 149, 48, 254, 215, 115, 151,
            115, 252, 183, 106, 22, 136, 0, 252, 211, 108, 84, 154, 176,
        ];

        let kex_hash: KexEcdhHash<X25519> = KexEcdhHash {
            client_identification: &client_identification,
            server_identification: &server_identification,
            client_kex_init: &client_kex_init,
            server_kex_init: &server_kex_init,
            server_host_key: &server_host_key,
            dh_client_key: &dh_client_key,
            dh_server_key: &dh_server_key,
            dh_secret: &dh_secret,
        };

        assert_eq!(kex_hash.sha256(), sha256_digest);
    }
}
