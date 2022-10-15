use super::method::*;
use super::msg::*;
use crate::identity::*;
use crate::transport::Message;
use crate::util::codec::*;
use crate::util::secret::*;

/// string    session identifier
/// byte      SSH_MSG_USERAUTH_REQUEST
/// string    user name
/// string    service name
/// string    "publickey"
/// boolean   TRUE
/// string    public key algorithm name
/// string    public key to be used for authentication
pub struct SignatureData<'a> {
    pub session_id: &'a Secret,
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub identity: &'a Identity,
}

impl<'a> SignatureData<'a> {
    pub fn new(
        session_id: &'a Secret,
        service_name: &'a str,
        user_name: &'a str,
        identity: &'a Identity,
    ) -> Self {
        Self {
            session_id,
            user_name,
            service_name,
            identity,
        }
    }
}

impl<'a> SshEncode for SignatureData<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push(self.session_id)?;
        e.push_u8(<MsgUserAuthRequest<PublicKeyMethod> as Message>::NUMBER)?;
        e.push_str_framed(&self.user_name)?;
        e.push_str_framed(&self.service_name)?;
        e.push_str_framed(<PublicKeyMethod as AuthMethod>::NAME)?;
        e.push_bool(true)?;
        e.push_str_framed(&self.identity.algorithm())?;
        e.push(self.identity)
    }
}
