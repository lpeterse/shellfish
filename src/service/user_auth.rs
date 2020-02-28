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

/// The `ssh-userauth` service negotiates and performs methods of user authentication between
/// client and server.
///
/// The service is a short-lived proxy that is only used to lift other services into an
/// authenticated context.
pub struct UserAuth {}

impl UserAuth {
    pub const NAME: &'static str = "ssh-userauth";

    /// Request another service with user authentication.
    pub async fn request<S: Socket, T: Service<Client>>(
        transport: Transport<Client, S>,
        config: &<Client as Role>::Config,
        user: &str,
        agent: Option<Agent>,
    ) -> Result<T, UserAuthError> {
        let mut t = transport.request_service(Self::NAME).await?;
        let service = <T as Service<Client>>::NAME;
        let agent = agent.ok_or(UserAuthError::NoMoreAuthMethods)?;
        let identities = agent.identities().await?;

        for (id, comment) in identities {
            log::debug!("Trying identity {}: {}", comment, id.algorithm());
            let success = match id {
                HostIdentity::Ed25519Key(x) => {
                    Self::try_pubkey::<S, SshEd25519>(&mut t, &agent, service, user, x).await?
                }
                HostIdentity::Ed25519Cert(x) => {
                    Self::try_pubkey::<S, SshEd25519Cert>(&mut t, &agent, service, user, x).await?
                }
                _ => false,
            };
            if success {
                return Ok(<T as Service<Client>>::new(config, t));
            }
        }

        Err(UserAuthError::NoMoreAuthMethods)
    }

    async fn try_pubkey<S: Socket, A>(
        transport: &mut Transport<Client, S>,
        agent: &Agent,
        service: &str,
        user: &str,
        id: A::Identity,
    ) -> Result<bool, UserAuthError>
    where
        A: AuthenticationAlgorithm,
        A::Identity: Clone + Encode,
        A::Signature: Encode + Decode,
    {
        let session_id = &transport.session_id();
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
        transport.send(&msg).await?;
        transport.flush().await?;
        transport.receive().await?;
        match transport.decode() {
            Some(x) => {
                let _: MsgSuccess = x;
                transport.consume();
                return Ok(true);
            }
            None => (),
        }
        let _: MsgFailure = transport.decode_ref().ok_or(TransportError::DecoderError)?;
        transport.consume();
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
