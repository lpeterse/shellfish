use super::msg::MsgKexInit;
use crate::identity::*;
use crate::transport::ident::*;
use crate::util::codec::*;
use crate::util::secret::Secret;
use sha2::{Digest, Sha256};

pub struct KexHash<'a, T1 = String, T2 = String> {
    pub client_id: &'a Identification<T1>,
    pub server_id: &'a Identification<T2>,
    pub client_kex_init: &'a MsgKexInit<T1>,
    pub server_kex_init: &'a MsgKexInit<T2>,
    pub server_host_key: &'a Identity,
    pub dh_client_key: &'a [u8],
    pub dh_server_key: &'a [u8],
    pub dh_secret: &'a Secret,
}

impl<'a, T1: AsRef<str>, T2: AsRef<str>> KexHash<'a, T1, T2> {
    #[must_use]
    pub fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_usize(SshCodec::size(self.client_id).ok()?)?;
        e.push(self.client_id)?;
        e.push_usize(SshCodec::size(self.server_id).ok()?)?;
        e.push(self.server_id)?;
        e.push_usize(SshCodec::size(self.client_kex_init).ok()?)?;
        e.push(self.client_kex_init)?;
        e.push_usize(SshCodec::size(self.server_kex_init).ok()?)?;
        e.push(self.server_kex_init)?;
        e.push(self.server_host_key)?;
        e.push_bytes_framed(self.dh_client_key)?;
        e.push_bytes_framed(self.dh_server_key)?;
        e.push_mpint(self.dh_secret.as_ref())
    }

    pub fn sha256(&self) -> Secret {
        let mut sha256 = Sha256::new();
        let _ = self.encode(&mut sha256);
        Secret::new(sha256.finalize_reset().as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::ssh_ed25519::*;
    use crate::transport::kex::*;

    #[test]
    fn kex_hash_01() {
        let client_id = Identification::new("hssh_0.1.0.0".into(), "".into());
        let server_id = Identification::<String>::new("hssh_0.1.0.0".into(), "".into());
        let client_kex_init = MsgKexInit {
            cookie: KexCookie([
                146, 105, 253, 96, 98, 147, 65, 76, 222, 166, 168, 241, 53, 43, 45, 168,
            ]),
            kex_algorithms: vec!["curve25519-sha256@libssh.org"],
            server_host_key_algorithms: vec!["ssh-ed25519".into()],
            encryption_algorithms_client_to_server: vec!["chacha20-poly1305@openssh.com"],
            encryption_algorithms_server_to_client: vec!["chacha20-poly1305@openssh.com"],
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
            kex_algorithms: vec!["curve25519-sha256@libssh.org".into()],
            server_host_key_algorithms: vec!["ssh-ed25519".into()],
            encryption_algorithms_client_to_server: vec!["chacha20-poly1305@openssh.com".into()],
            encryption_algorithms_server_to_client: vec!["chacha20-poly1305@openssh.com".into()],
            mac_algorithms_client_to_server: vec![],
            mac_algorithms_server_to_client: vec![],
            compression_algorithms_client_to_server: vec!["none".into()],
            compression_algorithms_server_to_client: vec!["none".into()],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        };
        let server_host_key = SshEd25519PublicKey(&[
            106, 114, 105, 46, 246, 21, 248, 172, 243, 187, 200, 45, 247, 246, 225, 218, 206, 250,
            145, 15, 246, 140, 131, 40, 234, 255, 135, 177, 8, 161, 128, 79,
        ]);
        let server_host_key = Identity::from(SshCodec::encode(&server_host_key).unwrap());
        let dh_client_key: [u8; 32] = [
            163, 184, 73, 53, 101, 235, 117, 249, 31, 97, 178, 63, 135, 35, 65, 5, 189, 180, 255,
            250, 242, 232, 76, 164, 186, 212, 21, 0, 223, 144, 162, 77,
        ];
        let dh_server_key: [u8; 32] = [
            236, 229, 149, 54, 50, 179, 149, 65, 53, 52, 47, 205, 191, 6, 241, 2, 134, 85, 228, 18,
            66, 201, 189, 121, 8, 17, 122, 81, 175, 192, 25, 58,
        ];
        let dh_secret = Secret::new(&[
            81, 115, 212, 227, 1, 156, 126, 179, 66, 238, 221, 162, 9, 2, 163, 168, 217, 121, 91,
            96, 227, 131, 212, 209, 11, 219, 182, 110, 136, 28, 151, 2,
        ]);
        let sha256_digest = [
            189, 219, 42, 55, 209, 120, 44, 65, 77, 213, 114, 209, 26, 149, 48, 254, 215, 115, 151,
            115, 252, 183, 106, 22, 136, 0, 252, 211, 108, 84, 154, 176,
        ];
        let kex_hash: KexHash<_, _> = KexHash {
            client_id: &client_id,
            server_id: &server_id,
            client_kex_init: &client_kex_init,
            server_kex_init: &server_kex_init,
            server_host_key: &server_host_key,
            dh_client_key: &dh_client_key,
            dh_server_key: &dh_server_key,
            dh_secret: &dh_secret,
        };

        assert_eq!(kex_hash.sha256().as_ref(), sha256_digest);
    }
}
