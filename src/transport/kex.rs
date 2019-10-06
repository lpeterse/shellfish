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

use super::config::*;
use super::*;

pub trait KexMachine {
    fn new(config: &TransportConfig) -> Self;
    fn init_local(&mut self);
    fn init_remote(&mut self, msg: MsgKexInit) -> Result<(), KexError>;
    fn is_init_sent(&self) -> bool;
    fn is_init_received(&self) -> bool;
    fn is_in_progress<T: Socket>(
        &mut self,
        cx: &mut Context,
        t: &mut Transmitter<T>,
    ) -> Result<bool, KexError>;
    fn consume<T: Socket>(&mut self, t: &mut Transmitter<T>) -> Result<(), KexError>;
    fn poll_flush<T: Socket>(
        &mut self,
        cx: &mut Context,
        t: &mut Transmitter<T>,
    ) -> Poll<Result<(), TransportError>>;
    fn session_id(&self) -> &Option<SessionId>;
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
