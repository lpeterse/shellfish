use super::*;

use async_std::os::unix::net::UnixStream;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

/// A client for the local `ssh-agent`.
#[derive(Debug, Clone)]
pub struct Agent {
    path: PathBuf,
}

impl Agent {
    const SSH_AUTH_SOCK: &'static str = "SSH_AUTH_SOCK";

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
    pub async fn identities(&self) -> Result<Vec<(Identity, String)>, AuthAgentError> {
        let mut t: Transmitter = UnixStream::connect(&self.path).await?.into();
        t.send(&MsgIdentitiesRequest {}).await?;
        t.receive::<MsgIdentitiesAnswer>()
            .await
            .map(|x| x.identities)
    }

    /// Sign a digest with the corresponding private key known to be owned be the agent.
    ///
    /// Returns `Ok(None)` in case the agent refused to sign.
    pub async fn sign(
        &self,
        identity: &Identity,
        data: &[u8],
    ) -> Result<Option<Signature>, AuthAgentError> {
        let msg = MsgSignRequest {
            identity,
            data,
            flags: 0,
        };
        let mut t: Transmitter = UnixStream::connect(&self.path).await?.into();
        t.send(&msg).await?;
        t.receive::<Result<MsgSignResponse, MsgFailure>>()
            .await
            .map(|x| x.ok().map(|y| y.signature))
    }
}

impl AuthAgent for Agent {
    fn identities(&self) -> BoxFuture<Result<Vec<(Identity, String)>, AuthAgentError>> {
        let self_ = self.clone();
        Box::pin(async move { Self::identities(&self_).await })
    }
    fn signature(
        &self,
        identity: &Identity,
        data: &[u8],
    ) -> BoxFuture<Result<Option<Signature>, AuthAgentError>> {
        let self_ = self.clone();
        let identity = identity.clone();
        let data = Vec::from(data);
        Box::pin(async move { Self::sign(&self_, &identity, &data).await })
    }
}
