mod client;
mod cookie;
mod hash;
mod server;

pub use self::client::*;
pub use self::cookie::*;
pub use self::hash::*;
pub use self::server::*;

use super::*;
use core::task::Poll;
use std::collections::VecDeque;

#[derive(Debug)]
pub enum KexMessage {
    Init(Arc<MsgKexInit<&'static str>>),
    EcdhInit(Arc<MsgEcdhInit>),
    EcdhReply(Arc<MsgEcdhReply>),
    NewKeys(Box<CipherConfig>),
}

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
    fn push_ecdh_init(&mut self, _msg: MsgEcdhInit) -> Result<(), TransportError> {
        Err(TransportError::InvalidState)
    }

    /// Push a [MsgKexEcdhReply] from peer into the state machine.
    ///
    /// Will raise an error if the state machine is not currently expecting this input.
    fn push_ecdh_reply(&mut self, _msg: MsgEcdhReply) -> Result<(), TransportError> {
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
