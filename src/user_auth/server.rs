use crate::transport::Transport;
use crate::util::codec::SshCodec;
use super::method::{PasswordMethod, AuthMethod, PublicKeyMethod};
use super::signature::SignatureData;
use super::{UserAuthSession, UserAuthError, AuthResult};
use super::msg::{MsgUserAuthBanner, MsgUserAuthRequest_, MsgUserAuthPkOk, MsgFailure, MsgSuccess};

pub async fn authenticate<Identity: Send + 'static>(
    mut transport: Transport,
    mut session: Box<dyn UserAuthSession<Identity = Identity>>,
) -> Result<Identity, UserAuthError> {
    // Send banner if configured
    if let Some(banner) = session.banner().await {
        let msg = MsgUserAuthBanner::new(banner);
        transport.send(&msg).await?;
        transport.flush().await?;
    }

    // 
    let identity = loop {
        let mut fail_partial_success = false;
        let mut fail_methods = vec![];

        let msg = transport.receive::<MsgUserAuthRequest_>().await?;
        match msg.method_name.as_str() {
            PasswordMethod::NAME => {
                let m: PasswordMethod = SshCodec::decode(&msg.method_blob)?;
                match session.try_password(msg.user_name, m.0).await {
                    AuthResult::Success(id) => break id,
                    AuthResult::Failure { methods, partial_success } => {
                        fail_methods = methods;
                        fail_partial_success = partial_success;
                    }
                }
            }
            PublicKeyMethod::NAME => {
                let m: PublicKeyMethod = SshCodec::decode(&msg.method_blob)?;
                let user = msg.user_name;
                let pkey = m.identity;
                if let Some(sig) = m.signature {
                    let data = SignatureData {
                        session_id: transport.session_id(),
                        user_name: &user,
                        service_name: &msg.service_name,
                        identity: &pkey,
                    };
                    let data = SshCodec::encode(&data)?;
                    if sig.verify(&pkey, &data).is_ok() {
                        match session.try_publickey(user, pkey).await {
                            AuthResult::Success(id) => break id,
                            AuthResult::Failure { methods, partial_success } => {
                                fail_methods = methods;
                                fail_partial_success = partial_success;
                            }
                        }
                    }
                } else {
                    if session.try_publickey_ok(user, pkey.clone()).await {
                        let msg = MsgUserAuthPkOk {
                            pk_algorithm: m.algorithm,
                            pk_blob: pkey.0,
                        };
                        transport.send(&msg).await?;
                        transport.flush().await?;
                        continue;
                    }
                }
            }
            _ => {}
        }
        let msg = MsgFailure::new(fail_partial_success, fail_methods);
        transport.send(&msg).await?;
        transport.flush().await?;
    };

    // Send success to client and return determined identity
    let msg = MsgSuccess;
    transport.send(&msg).await?;
    transport.flush().await?;
    Ok(identity)
}
