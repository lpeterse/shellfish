mod client;
mod server;

pub use self::client::*;
pub use self::cookie::*;
pub use self::ecdh::*;
pub use self::msg::*;
pub use self::server::*;

use super::keys::*;
use super::*;
use core::task::Poll;
use std::collections::VecDeque;

/// A state machine for key exchange.
pub trait Kex: std::fmt::Debug + Send {
    /// Initialize a key exchange (unless already in progress).
    fn init(&mut self);

    /// Push a [MsgKexInit] from peer into the state machine.
    ///
    /// Will raise an error unless the key exchange is in idle state or expecting
    /// the peer's kex init message.
    fn push_init(&mut self, msg: MsgKexInit) -> Result<(), TransportError>;

    /// Push a [MsgKexEcdhInit] from peer into the state machine.
    ///
    /// Will raise an error if the state machine is not currently expecting this input.
    fn push_ecdh_init(&mut self, _msg: MsgKexEcdhInit) -> Result<(), TransportError> {
        Err(TransportError::InvalidState)
    }

    /// Push a [MsgKexEcdhReply] from peer into the state machine.
    ///
    /// Will raise an error if the state machine is not currently expecting this input.
    fn push_ecdh_reply(&mut self, _msg: MsgKexEcdhReply) -> Result<(), TransportError> {
        Err(TransportError::InvalidState)
    }

    /// Push a [MsgNewKeys] from peer into the state machine.
    ///
    /// Will raise an error if the state machine is not currently expecting this input.
    fn push_new_keys(&mut self) -> Result<Box<CipherConfig>, TransportError>;

    /// Get the session id.
    ///
    /// Will raise an error if called before the first key exchange has been completed.
    fn session_id(&self) -> Option<&Secret>;

    /// Poll internal tasks and get a mutable reference on the outgoing messages queue.
    ///
    /// The messages in the queue shall be send to the peer in strict order and be
    /// removed from the queue after being sent.
    fn poll(&mut self, cx: &mut Context)
        -> Poll<Result<&mut VecDeque<KexMessage>, TransportError>>;
}

#[derive(Debug)]
pub enum KexMessage {
    Init(Arc<MsgKexInit<&'static str>>),
    EcdhInit(Arc<MsgKexEcdhInit>),
    EcdhReply(Arc<MsgKexEcdhReply>),
    NewKeys(Box<CipherConfig>),
}

pub fn ciphers<T1, T2>(
    common: fn(&[T2], &[T1]) -> Option<&'static str>,
    alg: KeyAlgorithm,
    server_init: &MsgKexInit<T1>,
    client_init: &MsgKexInit<T2>,
    k: &Secret,
    h: &Secret,
    sid: &Secret,
) -> Result<(CipherConfig, CipherConfig), TransportError> {
    const EENC: TransportError = TransportError::NoCommonEncryptionAlgorithm;
    const ECMP: TransportError = TransportError::NoCommonCompressionAlgorithm;

    let ea_c2s_c = &client_init.encryption_algorithms_client_to_server;
    let ea_c2s_s = &server_init.encryption_algorithms_client_to_server;
    let ea_c2s = common(ea_c2s_c, ea_c2s_s).ok_or(EENC)?;
    let ea_s2c_c = &client_init.encryption_algorithms_server_to_client;
    let ea_s2c_s = &server_init.encryption_algorithms_server_to_client;
    let ea_s2c = common(ea_s2c_c, ea_s2c_s).ok_or(EENC)?;
    let ca_c2s_c = &client_init.compression_algorithms_client_to_server;
    let ca_c2s_s = &server_init.compression_algorithms_client_to_server;
    let ca_c2s = common(ca_c2s_c, ca_c2s_s).ok_or(ECMP)?;
    let ca_s2c_c = &client_init.compression_algorithms_server_to_client;
    let ca_s2c_s = &server_init.compression_algorithms_server_to_client;
    let ca_s2c = common(ca_s2c_c, ca_s2c_s).ok_or(ECMP)?;
    let ma_c2s_c = &client_init.mac_algorithms_client_to_server;
    let ma_c2s_s = &server_init.mac_algorithms_client_to_server;
    let ma_c2s = common(ma_c2s_c, ma_c2s_s);
    let ma_s2c_c = &client_init.mac_algorithms_server_to_client;
    let ma_s2c_s = &server_init.mac_algorithms_server_to_client;
    let ma_s2c = common(ma_s2c_c, ma_s2c_s);
    let ks_c2s = KeyStream::new_c2s(alg, k, h, sid);
    let ks_s2c = KeyStream::new_s2c(alg, k, h, sid);
    let cc_c2s = CipherConfig::new(ea_c2s, ca_c2s, ma_c2s, ks_c2s);
    let cc_s2c = CipherConfig::new(ea_s2c, ca_s2c, ma_s2c, ks_s2c);

    Ok((cc_c2s, cc_s2c))
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

pub fn common(client: &[&'static str], server: &[String]) -> Option<&'static str> {
    for c in client {
        for s in server {
            if c == s {
                return Some(*c);
            }
        }
    }
    None
}

pub fn common_(client: &[String], server: &[&'static str]) -> Option<&'static str> {
    for c in client {
        for s in server {
            if c == s {
                return Some(*s);
            }
        }
    }
    None
}
