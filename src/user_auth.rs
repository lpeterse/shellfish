mod error;
mod method;
mod msg;
mod signature;

pub use self::error::*;

use self::method::*;
use self::msg::*;
use self::signature::*;
use crate::agent::*;
use crate::connection::{Connection, ConnectionConfig, ConnectionHandler};
use crate::identity::*;
use crate::transport::*;
use crate::util::codec::*;
use std::sync::Arc;

/// The `ssh-userauth` service negotiates and performs methods of user authentication between
/// client and server as described in RFC 4252.
///
/// The service is a short-lived proxy that is only used to lift other services into an
/// authenticated context.
pub struct UserAuth;

impl UserAuth {
    pub const SSH_USERAUTH: &'static str = "ssh-userauth";
    pub const SSH_CONNECTION: &'static str = "ssh-connection";

    /// Request another service with user authentication.
    pub async fn request_connection<F: FnOnce(&Connection) -> Box<dyn ConnectionHandler>>(
        transport: GenericTransport,
        config: &Arc<ConnectionConfig>,
        handle: F,
        user: &str,
        agent: &Arc<dyn AuthAgent>,
    ) -> Result<Connection, UserAuthError> {
        let mut t = transport.request_service(Self::SSH_USERAUTH).await?;
        let service = Self::SSH_CONNECTION;
        let identities = agent.identities().await?;

        for (id, comment) in identities {
            log::debug!("Trying identity: {} ({})", comment, id.algorithm());
            if Self::try_pubkey(&mut t, &agent, service, user, id).await? {
                return Ok(Connection::new(config, t, handle));
            }
        }

        Err(UserAuthError::NoMoreAuthMethods)
    }

    async fn try_pubkey(
        transport: &mut GenericTransport,
        agent: &Arc<dyn AuthAgent>,
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
        let data = SshCodec::encode(&data)?;
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
        transport.send(&msg).await?;
        transport.flush().await?;
        Ok(transport
            .receive::<Result<MsgSuccess, MsgFailure>>()
            .await?
            .is_ok())
    }
}
