mod msg_failure;
mod msg_identities_answer;
mod msg_identities_request;
mod msg_sign_request;
mod msg_sign_response;
mod msg_success;

use self::msg_failure::*;
use self::msg_identities_answer::*;
use self::msg_identities_request::*;
use self::msg_sign_request::*;
use self::msg_sign_response::*;

use crate::algorithm::authentication::*;
use crate::client::*;
use crate::codec::*;
use crate::role::*; 

use async_std::os::unix::net::UnixStream;
use futures::io::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;
use std::io::{Error};
use std::path::{Path, PathBuf};

pub struct Agent<R: Role> {
    phantom: std::marker::PhantomData<R>,
    path: PathBuf,
}

impl<R: Role> Agent<R> {
    const SSH_AUTH_SOCK: &'static str = "SSH_AUTH_SOCK";
}

impl Agent<Client> {
    /// Create a new agent instance by path.
    pub fn new(path: &Path) -> Self {
        Self {
            phantom: std::marker::PhantomData,
            path: path.into(),
        }
    }

    /// Create a new agent instance with the value
    /// of `SSH_AUTH_SOCK` as path.
    pub fn new_env() -> Option<Self> {
        let s = std::env::var_os(Self::SSH_AUTH_SOCK)?;
        Self {
            phantom: std::marker::PhantomData,
            path: TryFrom::try_from(s).ok()?,
        }
        .into()
    }

    /// Request a list of identities from the agent.
    pub async fn identities(&self) -> Result<Vec<(HostIdentity, String)>, AgentError> {
        let mut t = Transmitter::new(&self.path).await?;
        t.send(&MsgIdentitiesRequest {}).await?;
        t.receive::<MsgIdentitiesAnswer>().await.map(|x| x.identities)
    }

    /// Sign a digest with the corresponding private key known to be owned be the agent.
    pub async fn sign<S, D>(
        &self,
        identity: &S::Identity,
        data: &D,
        flags: S::SignatureFlags,
    ) -> Result<Option<S::Signature>, AgentError>
    where
        S: AuthenticationAlgorithm,
        S::Identity: Encode,
        S::Signature: Encode + Decode,
        D: Encode,
    {
        let msg: MsgSignRequest<S, D> = MsgSignRequest { key: identity, data, flags };
        let mut t = Transmitter::new(&self.path).await?;
        t.send(&msg).await?;
        let msg: E2<MsgSignResponse<S>, MsgFailure> = t.receive().await?;
        match msg {
            E2::A(x) => Ok(Some(x.signature)),
            E2::B(_) => Ok(None),
        }
    }
}

impl<R: Role> Clone for Agent<R> {
    fn clone(&self) -> Self {
        Self {
            phantom: std::marker::PhantomData,
            path: self.path.clone(),
        }
    }
}

#[derive(Debug)]
pub enum AgentError {
    IoError(Error),
    DecoderError
}

impl From<Error> for AgentError {
    fn from(e: Error) -> Self {
        Self::IoError(e)
    }
}

struct Transmitter {
    stream: UnixStream,
}

impl Transmitter {
    pub async fn new(path: &PathBuf) -> Result<Self, AgentError> {
        Ok(Self {
            stream: UnixStream::connect(&path).await?,
        })
    }

    pub async fn send<Msg: Encode>(&mut self, msg: &Msg) -> Result<(), AgentError> {
        let vec = BEncoder::encode(&Frame(&msg));
        self.stream.write_all(&vec).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn receive<Msg: Decode>(&mut self) -> Result<Msg, AgentError> {
        let mut len: [u8;4] = [0;4];
        self.stream.read_exact(&mut len[..]).await?;
        let len = u32::from_be_bytes(len) as usize;
        assert!(len <= 35000);
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, 0);
        self.stream.read_exact(&mut vec[..]).await?;
        BDecoder::decode(&vec).ok_or(AgentError::DecoderError)
    }
}

struct Frame<T> (T);

impl <T: Encode> Encode for Frame<T> {
    fn size(&self) -> usize {
        4 + self.0.size()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(self.0.size() as u32);
        self.0.encode(e);
    }
}
