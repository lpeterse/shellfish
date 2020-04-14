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
use crate::algorithm::auth::*;
use crate::codec::*;
use crate::service::Service;
use crate::transport::*;

use std::sync::Arc;

/// The `ssh-userauth` service negotiates and performs methods of user authentication between
/// client and server.
///
/// The service is a short-lived proxy that is only used to lift other services into an
/// authenticated context.
pub struct UserAuth {}

impl UserAuth {
    pub const NAME: &'static str = "ssh-userauth";

    /// Request another service with user authentication.
    pub async fn request<T: TransportLayer, S: Service>(
        transport: T,
        config: &Arc<<S as Service>::Config>,
        user: &str,
        agent: &Arc<dyn AuthAgent>,
    ) -> Result<S, UserAuthError> {
        let mut t = TransportLayerExt::request_service(transport, Self::NAME).await?;
        let service = <S as Service>::NAME;
        let identities = agent.identities().await?;

        for (id, comment) in identities {
            log::debug!("Trying identity: {} ({})", comment, id.algorithm());
            if Self::try_pubkey::<T>(&mut t, &agent, service, user, id).await? {
                return Ok(<S as Service>::new(config, t));
            }
        }

        Err(UserAuthError::NoMoreAuthMethods)
    }

    async fn try_pubkey<T: TransportLayer>(
        transport: &mut T,
        agent: &Arc<dyn AuthAgent>,
        service: &str,
        user: &str,
        identity: Identity,
    ) -> Result<bool, UserAuthError> {
        let session_id = &transport.session_id();
        let data = BEncoder::encode(&SignatureData {
            session_id,
            user_name: user,
            service_name: service,
            identity: &identity,
        });
        let signature = agent.signature(&identity, &data).await?;
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

#[derive(Debug)]
pub enum UserAuthError {
    TransportError(TransportError),
    AuthAgentError(AuthAgentError),
    NoMoreAuthMethods,
}

impl From<AuthAgentError> for UserAuthError {
    fn from(e: AuthAgentError) -> Self {
        Self::AuthAgentError(e)
    }
}

impl From<TransportError> for UserAuthError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}
