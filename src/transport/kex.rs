mod client;
mod server;

pub use self::client::*;
pub use self::cookie::*;
pub use self::ecdh_algorithm::*;
pub use self::ecdh_hash::*;
pub use self::msg_ecdh_init::*;
pub use self::msg_ecdh_reply::*;
pub use self::msg_kex_init::*;
pub use self::msg_new_keys::*;
pub use self::server::*;
pub use super::transmitter::*;

use super::*;

use futures::task::Poll;

/// A state machine for key exchange.
pub trait Kex {
    fn init(&mut self);
    fn is_active(&self) -> bool;
    fn is_sending_critical(&self) -> bool;
    fn is_receiving_critical(&self) -> bool;
    fn push_init(&mut self, msg: MsgKexInit) -> Result<(), TransportError>;
    fn push_ecdh_init(&mut self, msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError>;
    fn push_ecdh_reply(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError>;
    fn push_new_keys(&mut self) -> Result<CipherConfig, TransportError>;
    fn poll<F>(
        &mut self,
        cx: &mut Context,
        bytes_sent: u64,
        bytes_received: u64,
        f: F,
    ) -> Poll<Result<(), TransportError>>
    where
        F: FnMut(&mut Context, KexOutput) -> Poll<Result<(), TransportError>>;
    fn session_id(&self) -> &SessionId;
}

pub enum KexOutput {
    Init(MsgKexInit<&'static str>),
    EcdhInit(MsgKexEcdhInit<X25519>),
    EcdhReply(MsgKexEcdhReply<X25519>),
    NewKeys(EncryptionConfig),
}

#[derive(Clone, Debug)]
pub struct AlgorithmAgreement {
    pub ka: &'static str,
    pub ha: &'static str,
    pub ea_c2s: &'static str,
    pub ea_s2c: &'static str,
    pub ca_c2s: &'static str,
    pub ca_s2c: &'static str,
    pub ma_c2s: Option<&'static str>,
    pub ma_s2c: Option<&'static str>,
}

impl AlgorithmAgreement {
    pub fn agree(
        client_init: &MsgKexInit<&'static str>,
        server_init: &MsgKexInit,
    ) -> Result<AlgorithmAgreement, TransportError> {
        let ka = common(&client_init.kex_algorithms, &server_init.kex_algorithms);
        let ha = common(
            &client_init.server_host_key_algorithms,
            &server_init.server_host_key_algorithms,
        );
        let ea_c2s = common(
            &client_init.encryption_algorithms_client_to_server,
            &server_init.encryption_algorithms_client_to_server,
        );
        let ea_s2c = common(
            &client_init.encryption_algorithms_server_to_client,
            &server_init.encryption_algorithms_server_to_client,
        );
        let ma_c2s = common(
            &client_init.mac_algorithms_client_to_server,
            &server_init.mac_algorithms_client_to_server,
        );
        let ma_s2c = common(
            &client_init.mac_algorithms_server_to_client,
            &server_init.mac_algorithms_server_to_client,
        );
        let ca_c2s = common(
            &client_init.compression_algorithms_client_to_server,
            &server_init.compression_algorithms_client_to_server,
        );
        let ca_s2c = common(
            &client_init.compression_algorithms_server_to_client,
            &server_init.compression_algorithms_server_to_client,
        );

        Ok(Self {
            ka: ka.ok_or(TransportError::NoCommonKexAlgorithm)?,
            ha: ha.ok_or(TransportError::NoCommonServerHostKeyAlgorithm)?,
            ea_c2s: ea_c2s.ok_or(TransportError::NoCommonEncryptionAlgorithm)?,
            ea_s2c: ea_s2c.ok_or(TransportError::NoCommonEncryptionAlgorithm)?,
            ca_c2s: ca_c2s.ok_or(TransportError::NoCommonCompressionAlgorithm)?,
            ca_s2c: ca_s2c.ok_or(TransportError::NoCommonCompressionAlgorithm)?,
            ma_c2s,
            ma_s2c,
        })
    }
}

pub fn intersection(
    preferred: &Vec<&'static str>,
    supported: &[&'static str],
) -> Vec<&'static str> {
    preferred
        .iter()
        .filter_map(|p| {
            supported
                .iter()
                .find_map(|s| if p == s { Some(*s) } else { None })
        })
        .collect::<Vec<&'static str>>()
}

pub fn common(client: &Vec<&'static str>, server: &Vec<String>) -> Option<&'static str> {
    for c in client {
        for s in server {
            if c == s {
                return Some(*c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersection_01() {
        let xs = vec![];
        let ys = [];
        let zs: Vec<&'static str> = vec![];
        assert_eq!(intersection(&xs, &ys), zs)
    }

    #[test]
    fn test_intersection_02() {
        let xs = vec!["a", "b", "c"];
        let ys = ["k", "c", "t", "a"];
        let zs: Vec<&'static str> = vec!["a", "c"];
        assert_eq!(intersection(&xs, &ys), zs)
    }

    #[test]
    fn test_common_01() {
        let xs = vec![];
        let ys = vec![];
        assert_eq!(common(&xs, &ys), None)
    }

    #[test]
    fn test_common_02() {
        let xs = vec!["abc"];
        let ys = vec!["def".into()];
        assert_eq!(common(&xs, &ys), None)
    }

    #[test]
    fn test_common_03() {
        let xs = vec!["abc"];
        let ys = vec!["abc".into()];
        assert_eq!(common(&xs, &ys), Some("abc"))
    }

    #[test]
    fn test_common_04() {
        let xs = vec!["abc", "def"];
        let ys = vec!["def".into(), "abc".into()];
        assert_eq!(common(&xs, &ys), Some("abc"))
    }

    /*
    #[test]
    fn test_cipher_config_new_c2s_ok() {
        let ka = vec![];
        let ha = vec![];
        let ea = vec!["ea2".into(), "ea1".into()];
        let ma = vec!["ma2".into(), "ma1".into()];
        let ca = vec!["ca2".into(), "ca1".into()];
        let enc = vec!["ea1", "ea2"];
        let cmp = vec!["ca1", "ca2"];
        let mac = vec!["ma1", "ma2"];
        let cookie = KexCookie::random();
        let init = MsgKexInit::new(cookie, ka, ha, ea, ma, ca);
        let keys = KeyStreams::new_sha256(&[0][..], &[0][..], SessionId::default());
        match CipherConfig::new_client_to_server(&enc, &cmp, &mac, &init, keys) {
            Ok(x) => {
                assert_eq!(x.encryption_algorithm, "ea1");
                assert_eq!(x.compression_algorithm, "ca1");
                assert_eq!(x.mac_algorithm, Some("ma1"));
            }
            e => panic!("{:?}", e),
        }
    }*/

    #[test]
    fn test_cipher_config_new_s2c_01() {}
}
