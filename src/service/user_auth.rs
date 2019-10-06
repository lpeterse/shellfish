mod method;
mod msg_failure;
mod msg_success;
mod msg_userauth_request;
mod signature;

use self::method::*;
use self::msg_failure::*;
use self::msg_success::*;
use self::msg_userauth_request::*;
use self::signature::*;

use crate::service::Service;
use crate::agent::*;
use crate::algorithm::*;
use crate::client::*;
use crate::codec::*;
use crate::keys::*;
use crate::role::*;
use crate::transport::*;

pub struct UserAuth<R: Role> {
    phantom: std::marker::PhantomData<R>
}

impl <R: Role> Service for UserAuth<R> {
    const NAME: &'static str = "ssh-userauth";
}

impl UserAuth<Client> {
    pub async fn authenticate<S: Socket>(
        mut transport: Transport<Client, S>,
        service_name: &str,
        user_name: &str,
        agent: Option<Agent<Client>>,
    ) -> Result<Transport<Client, S>, UserAuthError> {
        match agent {
            None => (),
            Some(a) => {
                let identities = a.identities().await?;
                for (key, _comment) in identities {
                    match key {
                        PublicKey::Ed25519PublicKey(public_key) => {
                            let session_id = &transport.session_id().unwrap();
                            let data: SignatureData<SshEd25519> = SignatureData {
                                session_id,
                                user_name,
                                service_name,
                                public_key: public_key.clone(),
                            };
                            let signature: SshEd25519Signature = match a
                                .sign::<SshEd25519, SignatureData<SshEd25519>>(
                                    &public_key,
                                    &data,
                                    Default::default(),
                                )
                                .await?
                            {
                                None => continue,
                                Some(s) => s,
                            };
                            let req: MsgUserAuthRequest<PublicKeyMethod<SshEd25519>> =
                                MsgUserAuthRequest {
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
                            match transport.decode_ref().unwrap() {
                                // TODO
                                E2::A(x) => {
                                    let _: Success = x;
                                    transport.consume();
                                    return Ok(transport);
                                }
                                E2::B(x) => {
                                    let _: Failure = x;
                                    let name = <PublicKeyMethod<SshEd25519> as Method>::NAME;
                                    let b = !x.methods.contains(&name);
                                    transport.consume();
                                    if b {
                                        break;
                                    };
                                }
                            }
                        }
                        key => log::error!("Ignoring unsupported key {:?}", key),
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
