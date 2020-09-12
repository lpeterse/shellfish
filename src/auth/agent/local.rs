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
use super::*;

use crate::util::codec::*;

use async_std::os::unix::net::UnixStream;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

/// A client for the local `ssh-agent`.
#[derive(Debug, Clone)]
pub struct LocalAgent {
    path: PathBuf,
}

impl LocalAgent {
    const SSH_AUTH_SOCK: &'static str = "SSH_AUTH_SOCK";

    /// Create a new agent client by path designating the unix domain socket.
    pub fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    /// Create a new agent client using the value of `SSH_AUTH_SOCK` as path.
    pub fn new_env() -> Option<Self> {
        let s = std::env::var_os(Self::SSH_AUTH_SOCK)?;
        Self {
            path: TryFrom::try_from(s).ok()?,
        }
        .into()
    }
}

impl Agent for LocalAgent {
    fn identities(&self) -> AgentFuture<Vec<(Identity, String)>> {
        let self_ = self.clone();
        Box::pin(async move {
            let mut t: Transmitter = UnixStream::connect(&self_.path).await?.into();
            t.send(&MsgIdentitiesRequest {}).await?;
            t.receive::<MsgIdentitiesAnswer>()
                .await
                .map(|x| x.identities)
        })
    }

    fn signature(&self, identity: &Identity, data: &[u8], flags: u32) -> AgentFuture<Option<Signature>> {
        let self_ = self.clone();
        let identity = identity.clone();
        let data = Vec::from(data);
        Box::pin(async move {
            let msg = MsgSignRequest {
                identity: &identity,
                data: &data,
                flags,
            };
            let mut t: Transmitter = UnixStream::connect(&self_.path).await?.into();
            t.send(&msg).await?;
            t.receive::<Result<MsgSignResponse, MsgFailure>>()
                .await
                .map(|x| x.ok().map(|y| y.signature))
        })
    }
}
