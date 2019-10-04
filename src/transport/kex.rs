mod client;
mod cookie;
mod ecdh_algorithm;
mod ecdh_hash;
mod msg_ecdh_init;
mod msg_ecdh_reply;
mod msg_kex_init;
mod msg_new_keys;

pub use self::client::*;
pub use self::cookie::*;
pub use self::ecdh_algorithm::*;
pub use self::ecdh_hash::*;
pub use self::msg_ecdh_init::*;
pub use self::msg_ecdh_reply::*;
pub use self::msg_kex_init::*;
pub use self::msg_new_keys::*;
pub use super::transmitter::*;

use super::*;
use crate::algorithm::*;

pub trait KexMachine {
    fn new(interval_bytes: u64, interval_duration: std::time::Duration) -> Self;
    fn init_local(&mut self);
    fn init_remote(&mut self, msg: KexInit) -> Result<(), KexError>;
    fn is_init_sent(&self) -> bool;
    fn is_init_received(&self) -> bool;
    fn is_in_progress<T>(&mut self, cx: &mut Context, t: &mut Transmitter<T>) -> Result<bool, KexError>;
    fn consume<T: TransportStream>(&mut self, t: &mut Transmitter<T>) -> Result<(), KexError>;
    fn poll_flush<T: TransportStream>(
        &mut self,
        cx: &mut Context,
        t: &mut Transmitter<T>,
    ) -> Poll<Result<(), TransportError>>;
    fn session_id(&self) -> &SessionId;
}

#[derive(Debug)]
pub enum KexMsg {
    Init(KexInit),
    EcdhInit(KexEcdhInit<X25519>),
    EcdhReply(KexEcdhReply<X25519>),
    NewKeys(NewKeys),
}

#[derive(Debug)]
pub struct KexOutput {
    pub key_streams: KeyStreams,
    pub encryption_algorithm_client_to_server: EncryptionAlgorithm,
    pub encryption_algorithm_server_to_client: EncryptionAlgorithm,
    pub compression_algorithm_client_to_server: CompressionAlgorithm,
    pub compression_algorithm_server_to_client: CompressionAlgorithm,
    pub mac_algorithm_client_to_server: Option<MacAlgorithm>,
    pub mac_algorithm_server_to_client: Option<MacAlgorithm>,
}

pub fn common_algorithm<T: Clone + PartialEq>(client: &Vec<T>, server: &Vec<T>) -> Option<T> {
    for c in client {
        for s in server {
            if c == s {
                return Some(c.clone());
            }
        }
    }
    None
}

#[derive(Copy, Clone, Debug)]
pub enum KexError {
    IoError(std::io::ErrorKind),
    DecoderError,
    ProtocolError,
    NoCommonServerHostKeyAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorith,
    InvalidSignature,
}

impl From<std::io::Error> for KexError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
    }
}
