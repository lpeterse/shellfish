mod client;

pub use self::client::*;
pub use self::cookie::*;
pub use self::ecdh_algorithm::*;
pub use self::ecdh_hash::*;
pub use self::msg_ecdh_init::*;
pub use self::msg_ecdh_reply::*;
pub use self::msg_kex_init::*;
pub use self::msg_new_keys::*;
pub use super::transmitter::*;

use super::config::*;
use super::*;

use futures::task::Poll;

pub trait KexMachine {
    fn new<C: TransportConfig>(config: &C, remote_id: Identification<String>) -> Self;
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
        F: FnMut(&mut Context, &KexOutput) -> Poll<Result<(), TransportError>>;
    fn session_id(&self) -> &Option<SessionId>;
}

pub enum KexOutput {
    Init(MsgKexInit),
    EcdhInit(MsgKexEcdhInit<X25519>),
    EcdhReply(MsgKexEcdhReply<X25519>),
    NewKeys(CipherConfig),
}

pub type EncryptionConfig = CipherConfig;
pub type DecryptionConfig = CipherConfig;

#[derive(Clone, Debug)]
pub struct CipherConfig {
    pub encryption_algorithm: &'static str,
    pub compression_algorithm: &'static str,
    pub mac_algorithm: Option<&'static str>,
    pub key_streams: KeyStreams,
}

impl CipherConfig {
    pub fn new_client_to_server(
        enc: &Vec<&'static str>,
        comp: &Vec<&'static str>,
        mac: &Vec<&'static str>,
        server_init: &MsgKexInit,
        key_streams: KeyStreams,
    ) -> Result<Self, TransportError> {
        Ok(Self {
            encryption_algorithm: common(enc, &server_init.encryption_algorithms_client_to_server)
                .ok_or(TransportError::NoCommonEncryptionAlgorithm)?,
            compression_algorithm: common(
                comp,
                &server_init.compression_algorithms_client_to_server,
            )
            .ok_or(TransportError::NoCommonCompressionAlgorithm)?,
            mac_algorithm: common(mac, &server_init.mac_algorithms_client_to_server),
            key_streams,
        })
    }

    pub fn new_server_to_client(
        enc: &Vec<&'static str>,
        comp: &Vec<&'static str>,
        mac: &Vec<&'static str>,
        server_init: &MsgKexInit,
        key_streams: KeyStreams,
    ) -> Result<Self, TransportError> {
        Ok(Self {
            encryption_algorithm: common(enc, &server_init.encryption_algorithms_server_to_client)
                .ok_or(TransportError::NoCommonEncryptionAlgorithm)?,
            compression_algorithm: common(
                comp,
                &server_init.compression_algorithms_server_to_client,
            )
            .ok_or(TransportError::NoCommonCompressionAlgorithm)?,
            mac_algorithm: common(mac, &server_init.mac_algorithms_server_to_client),
            key_streams,
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
mod test {
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
}
