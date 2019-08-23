use tokio::prelude::*;
use tokio::net::unix::UnixStream;
use tokio::codec::Framed;

use self::codec::*;
use crate::keys::PublicKey;

mod codec;

//pub enum PublicKey {
//    Ed25519PublicKey([u8;32])
//}


pub enum Signature {
    Ed25519Signature([u8;64])
}

type Comment = String;
type AgentFuture<'a, T, E> = std::pin::Pin<Box<dyn Future<Output=Result<T,E>> + Send + 'a>>;

pub trait Agent {
    type Error;

    fn request_identities(&self) -> AgentFuture<'_, Vec<(PublicKey, Comment)>, Self::Error>;
    //fn sign_digest(&self, key: &PublicKey, digest: &[u8], flags: Flags) -> AgentFuture<Signature, Self::Error>;
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

impl LocalAgent {
    pub fn new() -> Self {
        Self {
            path: std::env::var_os("SSH_AUTH_SOCK")
        }
    }
}

impl Agent for LocalAgent {
    type Error = LocalAgentError;

    fn request_identities(&self) -> AgentFuture<'_, Vec<(PublicKey, Comment)>, Self::Error> {
        Box::pin(async move {
            match self.path.clone() {
                None => Err(LocalAgentError::EnvironmentVariableNotFound),
                Some(p) => {
                    let s = UnixStream::connect(std::path::Path::new(&p)).await?;
                    let mut s = Framed::new(s, AgentCodec::default());
                    s.send(AgentRequest::RequestIdentities).await?;
                    match s.into_future().map(|(x,_)| x).await {
                        None                                         => Err(LocalAgentError::ConnectionClosed),
                        Some(Ok(AgentResponse::IdentitiesAnswer(v))) => Ok(v),
                        Some(Ok(_))                                  => Err(LocalAgentError::ProtocolError("unexpected response")),
                        Some(Err(e))                                 => Err(LocalAgentError::from(e)),
                    }
                }
            }
        })
    }

    //fn sign_digest(&self, key: &PublicKey, digest: &[u8], flags: Flags) -> AgentFuture<Signature, Self::Error> {
    //    Box::new(futures::future::err(LocalAgentError::ConnectionClosed))
    //}
}