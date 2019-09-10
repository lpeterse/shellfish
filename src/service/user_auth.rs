mod failure;
mod method;
mod request;
mod success;

pub use self::failure::*;
pub use self::method::*;
pub use self::request::*;
pub use self::success::*;

use super::Service;
use crate::agent::*;
use crate::codec::*;
use crate::transport::*;

pub struct UserAuth {}

impl Service for UserAuth {
    const NAME: &'static str = "ssh-userauth";
}

impl UserAuth {
    pub async fn authenticate<T: TransportStream>(
        transport: &mut Transport<T>,
        service_name: &str,
        user_name: &str,
        agent: &mut Agent
    ) -> Result<(), UserAuthError> {

        transport.request_service(Self::NAME).await?;
        let identities = agent.identities().await?;
        for (public_key,_) in identities {
            let req: Request<Pubkey> = Request {
                user_name,
                service_name,
                method: Pubkey {
                    algorithm: &"",
                    public_key: public_key,
                    signature: None
                },
            };
            transport.send(&req).await?;
            transport.flush().await?;
            match transport.receive().await? {
                E2::A(x) => {
                    let _: Success = x;
                    return Ok(())
                },
                E2::B(x) => {
                    let _: Failure = x;
                    if !x.methods.contains(&Pubkey::NAME) { break };
                }
            }
        }
        Err(UserAuthError::NoMoreAuthMethods)
    }
}

#[derive(Debug)]
pub enum UserAuthError {
    NoMoreAuthMethods,
    AgentError(AgentError),
    TransportError(TransportError),
}

impl From<AgentError> for UserAuthError {
    fn from(e: AgentError) -> Self {
        Self::AgentError(e)
    }
}

impl From<TransportError> for UserAuthError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}
