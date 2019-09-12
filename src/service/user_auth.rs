mod failure;
mod method;
mod msg_userauth_request;
mod success;

pub use self::failure::*;
pub use self::method::*;
pub use self::msg_userauth_request::*;
pub use self::success::*;

use super::Service;
use crate::algorithm::*;
use crate::agent::*;
use crate::codec::*;
use crate::keys::*;
use crate::transport::*;

pub struct UserAuth {}

impl Service for UserAuth {
    const NAME: &'static str = "ssh-userauth";
}

impl UserAuth {
    pub async fn authenticate<T: TransportStream>(
        mut transport: Transport<T>,
        service_name: &str,
        user_name: &str,
        agent: Option<Agent>
    ) -> Result<Transport<T>, UserAuthError> {

        match agent {
            None => (),
            Some(a) => {
                let identities = a.identities().await?;
                for (key,_) in identities {
                    match key {
                        PublicKey::Ed25519PublicKey(key) => {
                            let req: MsgUserAuthRequest<PublicKeyMethod<SshEd25519>> = MsgUserAuthRequest {
                                user_name,
                                service_name,
                                method: PublicKeyMethod {
                                    public_key: key,
                                    signature: None
                                },
                            };
                            transport.send(&req).await?;
                            transport.flush().await?;
                            match transport.receive().await? {
                                E2::A(x) => {
                                    let _: Success = x;
                                    return Ok(transport)
                                },
                                E2::B(x) => {
                                    let _: Failure = x;
                                    let name = <PublicKeyMethod<SshEd25519> as Method>::NAME;
                                    if !x.methods.contains(&name) { break };
                                }
                            }
                        },
                        key => log::error!("Ignoring unsupported key {:?}", key)
                    }
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
