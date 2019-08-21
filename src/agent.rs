use tokio::prelude::*;
use tokio::net::unix::UnixStream;

use self::codec::*;

mod codec;

pub enum PublicKey {
    Ed25519PublicKey([u8;32])
}

pub enum Signature {
    Ed25519Signature([u8;64])
}

type Comment = String;
type Flags = u32;
type AgentFuture<T, E> = Box<dyn Future<Item = T, Error = E> + Send + 'static>;

pub trait Agent {
    type Error;

    fn get_identities(&self) -> AgentFuture<Vec<(PublicKey, Comment)>, Self::Error>;
    fn sign_digest(&self, key: &PublicKey, digest: &[u8], flags: Flags) -> AgentFuture<Signature, Self::Error>;
}

pub struct LocalAgent {
    path: Option<std::ffi::OsString>,
}

pub enum LocalAgentError {
    IoError(tokio::io::Error),
    EnvironmentVariableNotFound,
    FOOBAR,
}

impl From<tokio::io::Error> for LocalAgentError {
    fn from(e: tokio::io::Error) -> Self {
        LocalAgentError::IoError(e)
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

    fn get_identities(&self) -> AgentFuture<Vec<(PublicKey, Comment)>, Self::Error> {
        match self.path {
            None => Box::new(futures::future::err(LocalAgentError::EnvironmentVariableNotFound)),
            Some(ref p) => {
                let future = UnixStream::connect(std::path::Path::new(p))
                    .map_err(LocalAgentError::from)
                    .and_then(|sock| {
                        Ok(Vec::new())
                    }); 
                Box::new(future)
            }
        }
    }

    fn sign_digest(&self, key: &PublicKey, digest: &[u8], flags: Flags) -> AgentFuture<Signature, Self::Error> {
        Box::new(futures::future::err(LocalAgentError::FOOBAR))
    }
}