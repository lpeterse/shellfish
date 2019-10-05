mod method;
mod msg_failure;
mod msg_success;
mod msg_userauth_request;
mod signature;

pub use self::method::*;
pub use self::msg_failure::*;
pub use self::msg_success::*;
pub use self::msg_userauth_request::*;
pub use self::signature::*;

use super::Service;
use crate::algorithm::*;
use crate::client::*;
use crate::agent::*;
use crate::codec::*;
use crate::keys::*;
use crate::transport::*;
use crate::role::*;

pub struct UserAuth {}

impl Service for UserAuth {
    const NAME: &'static str = "ssh-userauth";
}

impl UserAuth {
    pub async fn authenticate<R: Role, T: TransportStream>(
        mut transport: Transport<R, T>,
        service_name: &str,
        user_name: &str,
        agent: Option<Agent<Client>>
    ) -> Result<Transport<R, T>, UserAuthError> {

        match agent {
            None => (),
            Some(a) => {
                let identities = a.identities().await?;
                for (key,_comment) in identities {
                    match key {
                        PublicKey::Ed25519PublicKey(public_key) => {
                            let data: SignatureData<SshEd25519> = SignatureData {
                                session_id: transport.session_id().as_ref(),
                                user_name,
                                service_name,
                                public_key: public_key.clone(),
                            };
                            let signature: SshEd25519Signature = match a.sign::<SshEd25519, SignatureData<SshEd25519>>(&public_key, &data, Default::default()).await? {
                                None => continue,
                                Some(s) => s,
                            };
                            let req: MsgUserAuthRequest<PublicKeyMethod<SshEd25519>> = MsgUserAuthRequest {
                                user_name,
                                service_name,
                                method: PublicKeyMethod {
                                    public_key,
                                    signature: Some(signature),
                                },
                            };
                            transport.send(&req).await?;
                            transport.flush().await?;
                            transport.receive().await?;
                            match transport.decode_ref().unwrap() { // TODO
                                E2::A(x) => {
                                    let _: Success = x;
                                    transport.consume();
                                    return Ok(transport)
                                },
                                E2::B(x) => {
                                    let _: Failure = x;
                                    let name = <PublicKeyMethod<SshEd25519> as Method>::NAME;
                                    let b =  !x.methods.contains(&name);
                                    transport.consume();
                                    if b { break };
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
