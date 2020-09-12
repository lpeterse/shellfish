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

use crate::auth::*;
use crate::connection::Connection;
use crate::transport::*;
use crate::util::codec::*;

use std::sync::Arc;

/// The `ssh-userauth` service negotiates and performs methods of user authentication between
/// client and server.
///
/// The service is a short-lived proxy that is only used to lift other services into an
/// authenticated context.
pub struct UserAuth {}

impl UserAuth {
    pub const NAME: &'static str = "ssh-userauth";

    pub async fn offer<S: Service>(
        t: S::Transport,
        c: Arc<<S as Service>::Config>,
    ) -> Result<UserAuthRequest<S>, UserAuthError> {
        let t = TransportLayerExt::offer_service(t, Self::NAME).await?;
        Ok(UserAuthRequest {
            t,
            c,
            username: "".into(),
        })
    }

    /// Request another service with user authentication.
    pub async fn request<S: Service>(
        transport: S::Transport,
        config: &Arc<<S as Service>::Config>,
        user: &str,
        agent: &Arc<dyn Agent>,
    ) -> Result<S, UserAuthError> {
        let mut t = TransportLayerExt::request_service(transport, Self::NAME).await?;
        let service = <S as Service>::NAME;
        let identities = agent.identities().await?;
        for (id, comment) in identities {
            log::debug!("Trying identity: {} ({})", comment, id.algorithm());
            if Self::try_pubkey::<S::Transport>(&mut t, &agent, service, user, id).await? {
                return Ok(<S as Service>::new(config, t));
            }
        }

        Err(UserAuthError::NoMoreAuthMethods)
    }

    async fn try_pubkey<T: TransportLayer>(
        transport: &mut T,
        agent: &Arc<dyn Agent>,
        service: &str,
        user: &str,
        identity: Identity,
    ) -> Result<bool, UserAuthError> {
        let session_id = transport.session_id()?;
        let data = SignatureData {
            session_id,
            user_name: user,
            service_name: service,
            identity: &identity,
        };
        let data = SliceEncoder::encode(&data);
        let signature = agent.signature(&identity, &data, 0).await?;
        if signature.is_none() {
            return Ok(false);
        }
        let msg = MsgUserAuthRequest::<PublicKeyMethod> {
            user_name: user,
            service_name: service,
            method: PublicKeyMethod {
                identity,
                signature,
            },
        };
        TransportLayerExt::send(transport, &msg).await?;
        TransportLayerExt::flush(transport).await?;
        TransportLayerExt::receive(transport).await?;
        if let Some(x) = transport.decode() {
            let _: MsgSuccess = x;
            transport.consume();
            return Ok(true);
        }
        let _: MsgFailure = transport.decode_ref().ok_or(TransportError::DecoderError)?;
        transport.consume();
        return Ok(false);
    }
}

pub struct UserAuthRequest<S: Service = Connection> {
    t: S::Transport,
    c: Arc<<S as Service>::Config>,
    username: String,
}

impl<S: Service> UserAuthRequest<S> {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn accept(self) -> S {
        todo!()
    }

    pub fn reject(self) {
        drop(self)
    }
}

#[derive(Debug)]
pub enum UserAuthError {
    TransportError(TransportError),
    AgentError(AgentError),
    NoMoreAuthMethods,
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
