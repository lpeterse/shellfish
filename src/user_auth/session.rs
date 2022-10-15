use crate::util::BoxFuture;
use crate::identity::Identity;

pub trait UserAuthSession: Send + Sync + 'static {
    type Identity: Send + 'static;

    fn banner(&self) -> BoxFuture<Option<String>> {
        Box::pin(async { None })
    }

    fn try_none(&mut self, username: String) -> BoxFuture<AuthResult<Self::Identity>> {
        let _ = username;
        Box::pin(async { AuthResult::failure(vec![]) })
    }

    fn try_publickey(
        &mut self,
        username: String,
        pubkey: Identity,
    ) -> BoxFuture<AuthResult<Self::Identity>> {
        let _ = username;
        let _ = pubkey;
        Box::pin(async { AuthResult::failure(vec![]) })
    }

    fn try_publickey_ok(
        &mut self,
        username: String,
        pubkey: Identity,
    ) -> BoxFuture<bool> {
        let _ = username;
        let _ = pubkey;
        Box::pin(async { true })
    }

    fn try_password(
        &mut self,
        username: String,
        password: String,
    ) -> BoxFuture<AuthResult<Self::Identity>> {
        let _ = username;
        let _ = password;
        Box::pin(async { AuthResult::failure(vec![]) })
    }
}

#[derive(Debug, Clone)]
pub enum AuthResult<Identity> {
    Success(Identity),
    Failure {
        methods: Vec<&'static str>,
        partial_success: bool
    }
}

impl <Identity> AuthResult<Identity> {
    fn success(identity: Identity) -> Self {
        Self::Success(identity)
    }

    fn failure(methods: Vec<&'static str>) -> Self {
        Self::Failure { methods, partial_success: false }
    }

    fn failure_partial_success(methods: Vec<&'static str>) -> Self {
        Self::Failure { methods, partial_success: true }
    }
}
