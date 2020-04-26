mod agent;
mod error;
mod frame;
mod msg_failure;
mod msg_identities_answer;
mod msg_identities_request;
mod msg_sign_request;
mod msg_sign_response;
mod msg_success;
mod transmitter;

pub use self::agent::*;
pub use self::error::*;

use self::frame::*;
use self::msg_failure::*;
use self::msg_identities_answer::*;
use self::msg_identities_request::*;
use self::msg_sign_request::*;
use self::msg_sign_response::*;
use self::transmitter::*;

use crate::algorithm::auth::*;
use crate::codec::*;
use crate::util::*;

pub trait AuthAgent: std::fmt::Debug + Send + Sync + 'static {
    fn identities(&self) -> BoxFuture<Result<Vec<(Identity, String)>, AuthAgentError>>;
    fn signature(
        &self,
        identity: &Identity,
        data: &[u8],
    ) -> BoxFuture<Result<Option<Signature>, AuthAgentError>>;
}

impl AuthAgent for () {
    fn identities(&self) -> BoxFuture<Result<Vec<(Identity, String)>, AuthAgentError>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn signature(
        &self,
        _: &Identity,
        _: &[u8],
    ) -> BoxFuture<Result<Option<Signature>, AuthAgentError>> {
        Box::pin(async { Ok(None) })
    }
}
