use super::method::{AuthMethod, NoneMethod, PasswordMethod, PublicKeyMethod};
use super::msg::{MsgFailure, MsgSuccess, MsgUserAuthBanner, MsgUserAuthPkOk, MsgUserAuthRequest_};
use super::signature::SignatureData;
use super::{AuthResult, UserAuthError, UserAuthSession};
use crate::transport::{DisconnectReason, MsgDisconnect, Transport};
use crate::util::codec::SshCodec;

pub async fn authenticate<Identity: Send + 'static>(
    mut session: Box<dyn UserAuthSession<Identity = Identity>>,
    transport: &mut Transport,
    service: &'static str,
) -> Result<Identity, UserAuthError> {
    let s = &mut session;
    let t = transport;

    if let Some(banner) = s.banner().await {
        let msg = MsgUserAuthBanner::new(banner);
        t.send(&msg).await?;
        t.flush().await?;
    }

    macro_rules! reply {
        ( $x:expr ) => {
            match $x {
                AuthResult::Success { identity } => {
                    send_success(t).await?;
                    return Ok(identity);
                }
                AuthResult::Failure { partial_success } => {
                    send_failure(t, partial_success, s.methods()).await?;
                }
                AuthResult::Disconnect => {
                    send_disconnect(t).await?;
                }
            }
        };
    }

    loop {
        let msg = t.receive::<MsgUserAuthRequest_>().await?;
        let srv = msg.service_name;
        let user = msg.user_name;

        if service != srv {
            send_disconnect_service_not_available(t).await?;
        }

        match msg.method_name.as_str() {
            NoneMethod::NAME => {
                reply!(s.try_none(user).await)
            }
            PasswordMethod::NAME => {
                let m: PasswordMethod = SshCodec::decode(&msg.method_blob)?;
                let pass = m.0;
                reply!(s.try_password(user, pass).await)
            }
            PublicKeyMethod::NAME => {
                let m: PublicKeyMethod = SshCodec::decode(&msg.method_blob)?;
                let pkey = m.identity;
                if let Some(sig) = m.signature {
                    let sid = t.session_id();
                    let data = SignatureData::new(sid, &srv, &user, &pkey);
                    let data = SshCodec::encode(&data)?;
                    if sig.verify(&pkey, &data).is_ok() {
                        reply!(s.try_publickey(user, pkey).await)
                    } else {
                        send_failure(t, false, s.methods()).await?;
                    }
                } else {
                    match s.try_publickey_ok(user, pkey.clone()).await {
                        AuthResult::Success { .. } => {
                            send_pk_ok(t, m.algorithm, pkey.0).await?;
                        }
                        AuthResult::Failure { .. } => {
                            send_failure(t, false, s.methods()).await?;
                        }
                        AuthResult::Disconnect => {
                            send_disconnect(t).await?;
                        }
                    }
                }
            }
            _ => send_failure(t, false, s.methods()).await?,
        }
    }
}

async fn send_pk_ok(
    t: &mut Transport,
    pk_algorithm: String,
    pk_blob: Vec<u8>,
) -> Result<(), UserAuthError> {
    let msg = MsgUserAuthPkOk {
        pk_algorithm,
        pk_blob,
    };
    t.send(&msg).await?;
    t.flush().await?;
    Ok(())
}

async fn send_success(t: &mut Transport) -> Result<(), UserAuthError> {
    let msg = MsgSuccess;
    t.send(&msg).await?;
    t.flush().await?;
    Ok(())
}

async fn send_failure(
    t: &mut Transport,
    partial_success: bool,
    methods: Vec<&'static str>,
) -> Result<(), UserAuthError> {
    let msg = MsgFailure::new(partial_success, methods);
    t.send(&msg).await?;
    t.flush().await?;
    Ok(())
}

async fn send_disconnect(t: &mut Transport) -> Result<(), UserAuthError> {
    let msg = MsgDisconnect::new(DisconnectReason::NO_MORE_AUTH_METHODS_AVAILABLE);
    t.send(&msg).await?;
    t.flush().await?;
    Err(UserAuthError::NoMoreAuthMethods)
}

async fn send_disconnect_service_not_available(t: &mut Transport) -> Result<(), UserAuthError> {
    let msg = MsgDisconnect::new(DisconnectReason::SERVICE_NOT_AVAILABLE);
    t.send(&msg).await?;
    t.flush().await?;
    Err(UserAuthError::ServiceNotAvailable)
}
