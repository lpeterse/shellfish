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

use super::*;

use async_std::task::Poll;

/// A state machine for key exchange.
pub trait Kex {
    fn init(&mut self, tx: u64, rx: u64);
    fn is_active(&self) -> bool;
    fn is_sending_critical(&self) -> bool;
    fn is_receiving_critical(&self) -> bool;

    fn poll_init(
        &mut self,
        cx: &mut Context,
        tx: u64,
        rx: u64,
    ) -> Poll<Result<MsgKexInit<&'static str>, TransportError>>;
    fn push_init_tx(&mut self) -> Result<(), TransportError>;
    fn push_init_rx(&mut self, tx: u64, rx: u64, msg: MsgKexInit) -> Result<(), TransportError>;

    fn poll_ecdh_init(&mut self, cx: &mut Context) -> Poll<Result<MsgKexEcdhInit<X25519>, TransportError>>;
    fn push_ecdh_init_tx(&mut self) -> Result<(), TransportError>;
    fn push_ecdh_init_rx(&mut self, msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError>;

    fn poll_ecdh_reply(&mut self, cx: &mut Context) -> Poll<Result<MsgKexEcdhReply<X25519>, TransportError>>;
    fn push_ecdh_reply_tx(&mut self) -> Result<(), TransportError>;
    fn push_ecdh_reply_rx(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError>;

    fn poll_new_keys_tx(&mut self, cx: &mut Context) -> Poll<Result<EncryptionConfig, TransportError>>;
    fn poll_new_keys_rx(&mut self, cx: &mut Context) -> Poll<Result<DecryptionConfig, TransportError>>;
    fn push_new_keys_tx(&mut self) -> Result<(), TransportError>;
    fn push_new_keys_rx(&mut self) -> Result<(), TransportError>;

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

    #[test]
    fn test_algorithm_agreement_01() {
        let client_init = MsgKexInit::<&'static str> {
            cookie: KexCookie::random(),
            kex_algorithms: vec!["ka", "ka_"],
            server_host_key_algorithms: vec!["ha", "ha_"],
            encryption_algorithms_client_to_server: vec!["ea_c2s", "ea_c2s_"],
            encryption_algorithms_server_to_client: vec!["ea_s2c", "ea_s2c_"],
            mac_algorithms_client_to_server: vec!["ma_c2s", "ma_c2s_"],
            mac_algorithms_server_to_client: vec!["ma_s2c", "ma_s2c_"],
            compression_algorithms_client_to_server: vec!["ca_c2s", "ca_c2s_"],
            compression_algorithms_server_to_client: vec!["ca_s2c", "ca_s2c_"],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        };
        let server_init = MsgKexInit {
            cookie: KexCookie::random(),
            kex_algorithms: vec!["ka_".into(), "ka".into()],
            server_host_key_algorithms: vec!["ha_".into(), "ha".into()],
            encryption_algorithms_client_to_server: vec!["ea_c2s_".into(), "ea_c2s".into()],
            encryption_algorithms_server_to_client: vec!["ea_s2c_".into(), "ea_s2c".into()],
            mac_algorithms_client_to_server: vec!["ma_c2s_".into(), "ma_c2s".into()],
            mac_algorithms_server_to_client: vec!["ma_s2c_".into(), "ma_s2c".into()],
            compression_algorithms_client_to_server: vec!["ca_c2s_".into(), "ca_c2s".into()],
            compression_algorithms_server_to_client: vec!["ca_s2c_".into(), "ca_s2c".into()],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        };
        let x = AlgorithmAgreement::agree(&client_init, &server_init).unwrap();
        assert_eq!(x.ka, "ka");
        assert_eq!(x.ha, "ha");
        assert_eq!(x.ea_c2s, "ea_c2s");
        assert_eq!(x.ea_s2c, "ea_s2c");
        assert_eq!(x.ca_c2s, "ca_c2s");
        assert_eq!(x.ca_s2c, "ca_s2c");
        assert_eq!(x.ma_c2s, Some("ma_c2s"));
        assert_eq!(x.ma_s2c, Some("ma_s2c"));
    }
}
