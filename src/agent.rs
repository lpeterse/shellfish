mod local;
mod internal;

pub use self::local::*;
pub use self::internal::*;

use crate::identity::*;
use crate::util::*;
use std::error::Error;
use std::sync::Arc;

pub type AuthAgentResult<T> = Result<T, AuthAgentError>;
pub type AuthAgentFuture<T> = BoxFuture<AuthAgentResult<T>>;

/// This trait describes the methods of `ssh-agent`.
pub trait AuthAgent: std::fmt::Debug + Send + Sync + 'static {
    /// Request a list of identities from the agent.
    fn identities(&self) -> AuthAgentFuture<Vec<(Identity, String)>>;
    /// Sign a digest with the corresponding private key known to be owned be the agent.
    ///
    /// Returns `Ok(None)` in case the agent refused to sign.
    fn signature(
        &self,
        id: &Identity,
        data: &[u8],
        flags: u32,
    ) -> AuthAgentFuture<Option<Signature>>;
}

/// The unit agent neither offers identities nor will it sign anything.
impl AuthAgent for () {
    fn identities(&self) -> AuthAgentFuture<Vec<(Identity, String)>> {
        Box::pin(async { Ok(vec![]) })
    }

    fn signature(&self, _: &Identity, _: &[u8], _: u32) -> AuthAgentFuture<Option<Signature>> {
        Box::pin(async { Ok(None) })
    }
}

#[derive(Clone, Debug)]
pub struct AuthAgentError(ArcError);

impl AuthAgentError {
    pub fn new<E: Error + Send + Sync + 'static>(e: E) -> Self {
        Self(Arc::new(e))
    }
}

impl Error for AuthAgentError {}

impl std::fmt::Display for AuthAgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<std::io::Error> for AuthAgentError {
    fn from(e: std::io::Error) -> Self {
        Self(Arc::new(e))
    }
}
