mod local;

pub use self::local::*;

use super::*;
use crate::util::*;

pub type AgentError = BoxError;
pub type AgentResult<T> = Result<T, BoxError>;
pub type AgentFuture<T> = BoxFuture<AgentResult<T>>;

/// This trait describes the methods of `ssh-agent`.
pub trait Agent: std::fmt::Debug + Send + Sync + 'static {
    /// Request a list of identities from the agent.
    fn identities(&self) -> AgentFuture<Vec<(Identity, String)>>;
    /// Sign a digest with the corresponding private key known to be owned be the agent.
    ///
    /// Returns `Ok(None)` in case the agent refused to sign.
    fn signature(&self, id: &Identity, data: &[u8], flags: u32) -> AgentFuture<Option<Signature>>;
}

/// The unit agent neither offers identities nor will it sign anything.
impl Agent for () {
    fn identities(&self) -> AgentFuture<Vec<(Identity, String)>> {
        Box::pin(async { Ok(vec![]) })
    }

    fn signature(&self, _: &Identity, _: &[u8], _: u32) -> AgentFuture<Option<Signature>> {
        Box::pin(async { Ok(None) })
    }
}
