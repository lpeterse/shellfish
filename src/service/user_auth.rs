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

use crate::agent::*;
use crate::algorithm::authentication::*;
use crate::client::*;
use crate::codec::*;
use crate::role::*;
use crate::service::Service;
use crate::transport::*;

use async_std::net::TcpStream;

/// The `ssh-userauth` service negotiates and performs methods of user authentication between
/// client and server.
///
/// The service is a short-lived proxy that is only used to lift other services into an
/// authenticated context.
pub struct UserAuth<R: Role> {
    transport: Transport<R, TcpStream>,
}

impl<R: Role> Service<R> for UserAuth<R> {
    const NAME: &'static str = "ssh-userauth";

    fn new(transport: Transport<R, TcpStream>) -> Self {
        Self { transport }
    }
}

impl UserAuth<Client> {
    /// Request another service with user authentication.
    pub async fn authenticate<S: Service<Client>>(
        mut self,
        user: &str,
        agent: Option<Agent<Client>>,
    ) -> Result<S, UserAuthError> {
        let service = <S as Service<Client>>::NAME;
        let agent = agent.ok_or(UserAuthError::NoMoreAuthMethods)?;
        let identities = agent.identities().await?;

        for (id, comment) in identities {
            log::debug!("Trying identity {}: {}", comment, id.algorithm());
            let success = match id {
                HostIdentity::Ed25519Key(x) => {
                    (&mut self)
                        .try_pubkey::<SshEd25519>(&agent, service, user, x)
                        .await?
                }
                HostIdentity::Ed25519Cert(x) => {
                    (&mut self)
                        .try_pubkey::<SshEd25519Cert>(&agent, service, user, x)
                        .await?
                }
                _ => false,
            };
            if success {
                return Ok(<S as Service<Client>>::new(self.transport));
            }
        }

        Err(UserAuthError::NoMoreAuthMethods)
    }

    async fn try_pubkey<A>(
        &mut self,
        agent: &Agent<Client>,
        service: &str,
        user: &str,
        id: A::Identity,
    ) -> Result<bool, UserAuthError>
    where
        A: AuthenticationAlgorithm,
        A::Identity: Clone + Encode,
        A::Signature: Decode,
    {
        let session_id = &self.transport.session_id().unwrap();
        let data: SignatureData<A> = SignatureData {
            session_id,
            user_name: user,
            service_name: service,
            public_key: id.clone(),
        };
        let signature = agent.sign::<A, _>(&id, &data, Default::default()).await?;
        let signature = match signature {
            None => return Ok(false),
            Some(s) => s,
        };
        let msg = MsgUserAuthRequest::<PublicKeyMethod<A>> {
            user_name: user,
            service_name: service,
            method: PublicKeyMethod {
                public_key: id,
                signature: Some(signature),
            },
        };
        self.transport.send(&msg).await?;
        self.transport.flush().await?;
        self.transport.receive().await?;
        match self.transport.decode() {
            Some(x) => {
                let _: MsgSuccess = x;
                self.transport.consume();
                return Ok(true);
            }
            None => (),
        }
        let _: MsgFailure = self
            .transport
            .decode_ref()
            .ok_or(TransportError::DecoderError)?;
        self.transport.consume();
        return Ok(false);
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
