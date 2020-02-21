mod frame;
mod msg_failure;
mod msg_identities_answer;
mod msg_identities_request;
mod msg_sign_request;
mod msg_sign_response;
mod msg_success;
mod transmitter;

use self::frame::*;
use self::msg_failure::*;
use self::msg_identities_answer::*;
use self::msg_identities_request::*;
use self::msg_sign_request::*;
use self::msg_sign_response::*;
use self::transmitter::*;

use crate::algorithm::authentication::*;
use crate::codec::*;

use std::convert::TryFrom;
use std::io::Error;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Agent {
    path: PathBuf,
}

impl Agent {
    const SSH_AUTH_SOCK: &'static str = "SSH_AUTH_SOCK";
}

impl Agent {
    /// Create a new agent client by path designating the unix domain socket.
    pub fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    /// Create a new agent client with the value of `SSH_AUTH_SOCK` as path.
    pub fn new_env() -> Option<Self> {
        let s = std::env::var_os(Self::SSH_AUTH_SOCK)?;
        Self {
            path: TryFrom::try_from(s).ok()?,
        }
        .into()
    }

    /// Request a list of identities from the agent.
    pub async fn identities(&self) -> Result<Vec<(HostIdentity, String)>, AgentError> {
        let mut t = Transmitter::new(&self.path).await?;
        t.send(&MsgIdentitiesRequest {}).await?;
        t.receive::<MsgIdentitiesAnswer>()
            .await
            .map(|x| x.identities)
    }

    /// Sign a digest with the corresponding private key known to be owned be the agent.
    ///
    /// Returns `Ok(None)` in case the agent refused to sign.
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
        let msg: MsgSignRequest<S, D> = MsgSignRequest {
            key: identity,
            data,
            flags,
        };
        let mut t = Transmitter::new(&self.path).await?;
        t.send(&msg).await?;
        let msg: E2<MsgSignResponse<S>, MsgFailure> = t.receive().await?;
        match msg {
            E2::A(x) => Ok(Some(x.signature)),
            E2::B(_) => Ok(None),
        }
    }
}

#[derive(Debug)]
pub enum AgentError {
    IoError(Error),
    FrameError,
    DecoderError,
}

impl From<Error> for AgentError {
    fn from(e: Error) -> Self {
        Self::IoError(e)
    }
}
