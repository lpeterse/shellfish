use tokio::prelude::*;
use tokio::net::unix::UnixStream;
use tokio::codec::Framed;

use self::codec::*;

mod codec;

//pub enum PublicKey {
//    Ed25519PublicKey([u8;32])
//}

pub type PublicKey = Vec<u8>;

pub enum Signature {
    Ed25519Signature([u8;64])
}

type Comment = String;
type Flags = u32;
type AgentFuture<T, E> = Box<dyn Future<Item = T, Error = E> + Send + 'static>;

pub trait Agent {
    type Error;

    fn request_identities(&self) -> AgentFuture<Vec<(PublicKey, Comment)>, Self::Error>;
    fn sign_digest(&self, key: &PublicKey, digest: &[u8], flags: Flags) -> AgentFuture<Signature, Self::Error>;
}

pub struct LocalAgent {
    path: Option<std::ffi::OsString>,
}

#[derive(Debug)]
pub enum LocalAgentError {
    IoError(tokio::io::Error),
    EnvironmentVariableNotFound,
    CodecError(AgentCodecError),
    ProtocolError(&'static str),
    ConnectionClosed,
}

impl From<tokio::io::Error> for LocalAgentError {
    fn from(e: tokio::io::Error) -> Self {
        LocalAgentError::IoError(e)
    }
}

impl From<AgentCodecError> for LocalAgentError {
    fn from(e: AgentCodecError) -> Self {
        LocalAgentError::CodecError(e)
    }
}

impl <T> From<(AgentCodecError,T)> for LocalAgentError {
    fn from((e, _): (AgentCodecError, T)) -> Self {
        LocalAgentError::CodecError(e)
    }
}

impl LocalAgent {
    pub fn new() -> Self {
        Self {
            path: std::env::var_os("SSH_AUTH_SOCK")
        }
    }
}

impl Agent for LocalAgent {
    type Error = LocalAgentError;

    fn request_identities(&self) -> AgentFuture<Vec<(PublicKey, Comment)>, Self::Error> {
        match self.path {
            None => Box::new(futures::future::err(LocalAgentError::EnvironmentVariableNotFound)),
            Some(ref p) => {
                let future = UnixStream::connect(std::path::Path::new(p))
                    .map_err(LocalAgentError::from)
                    .map(|s| Framed::new(s, AgentCodec::default()))
                    .and_then(|s| s.send(AgentRequest::RequestIdentities).map_err(LocalAgentError::from))
                    .and_then(|s| s.into_future().map_err(LocalAgentError::from))
                    .and_then(|(x,_)| match x {
                        Some(AgentResponse::IdentitiesAnswer(v)) => Ok(v),
                        None                                     => Err(LocalAgentError::ConnectionClosed),
                        _                                        => Err(LocalAgentError::ProtocolError("unexpected response")),
                    });
                Box::new(future)
            }
        }
    }

    fn sign_digest(&self, key: &PublicKey, digest: &[u8], flags: Flags) -> AgentFuture<Signature, Self::Error> {
        Box::new(futures::future::err(LocalAgentError::ConnectionClosed))
    }
}