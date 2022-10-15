use crate::identity::Identity;
use crate::util::BoxFuture;

pub trait UserAuthSession: Send + Sync + 'static {
    type Identity: Send + 'static;

    fn methods(&self) -> Vec<&'static str>;

    fn banner(&self) -> BoxFuture<Option<String>> {
        Box::pin(async { None })
    }

    fn try_none(&mut self, username: String) -> BoxFuture<AuthResult<Self::Identity>> {
        let _ = username;
        Box::pin(async { AuthResult::failure(false) })
    }

    fn try_publickey(
        &mut self,
        username: String,
        pubkey: Identity,
    ) -> BoxFuture<AuthResult<Self::Identity>> {
        let _ = username;
        let _ = pubkey;
        Box::pin(async { AuthResult::failure(false) })
    }

    fn try_publickey_ok(&mut self, username: String, pubkey: Identity) -> BoxFuture<AuthResult<()>> {
        let _ = username;
        let _ = pubkey;
        Box::pin(async { AuthResult::failure(false) })
    }

    fn try_password(
        &mut self,
        username: String,
        password: String,
    ) -> BoxFuture<AuthResult<Self::Identity>> {
        let _ = username;
        let _ = password;
        Box::pin(async { AuthResult::failure(false) })
    }
}

#[derive(Debug, Clone)]
pub enum AuthResult<Identity> {
    Success { identity: Identity },
    Failure { partial_success: bool },
    Disconnect
}

impl<Identity> AuthResult<Identity> {
    pub fn success(identity: Identity) -> Self {
        Self::Success { identity }
    }

    pub fn failure(partial_success: bool) -> Self {
        Self::Failure { partial_success }
    }

    pub fn disconnect() -> Self {
        Self::Disconnect
    }
}
