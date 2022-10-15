use shellfish::agent::*;
use shellfish::connection::Connection;
use shellfish::server::*;
use shellfish::user_auth::{UserAuthSession, AuthResult};
use shellfish::util::BoxFuture;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    tokio::runtime::Runtime::new()?.block_on(main_async())
}

async fn main_async() -> Result<(), Box<dyn Error>> {
    let config = Arc::new(ServerConfig::default());
    let handler = Arc::new(MyServerHandler::new());
    let auth_agent = Arc::new(InternalAgent::new_random());
    Server::run(config, handler, auth_agent).await?;
    Ok(())
}

struct MyServerHandler {}

impl MyServerHandler {
    fn new() -> Self {
        Self {}
    }
}

impl ServerHandler for MyServerHandler {
    type Identity = String;

    fn on_accept(&self, addr: SocketAddr) -> BoxFuture<Option<Box<dyn UserAuthSession<Identity = Self::Identity>>>> {
        log::info!("New connection: {}", addr);
        let x: Box<dyn UserAuthSession<Identity = String>> = Box::new(MyUserAuthProvider);
        Box::pin(async { Some(x) })
    }

    fn on_connection(&self, connection: Connection, identity: Self::Identity) {
        log::info!("New connection: {:?} {:?}", connection, identity);
    }

    fn on_error(&self, err: ServerError) {
        log::error!("{}", err);
    }
}

#[derive(Debug)]
pub struct MyUserAuthProvider;

impl UserAuthSession for MyUserAuthProvider {
    type Identity = String;

    fn methods(&self) -> Vec<&'static str> {
        vec!["password", "publickey"]
    }

    fn banner(&self) -> BoxFuture<Option<String>> {
        Box::pin(async {
            Some("+++\r\nHallo Welt!\r\nIch bin ein Server!\r\nIch hoffe, du hast Spaß am Gerät!\r\n+++\r\n".into())
        })
    }

    fn try_none(&mut self, username: String) -> BoxFuture<AuthResult<Self::Identity>> {
        log::info!("TRY NONE");
        let _ = username;
        Box::pin(async { AuthResult::failure(false) })
    }

    fn try_password(
        &mut self,
        username: String,
        password: String,
    ) -> BoxFuture<AuthResult<Self::Identity>> {
        log::info!("TRY PASSWORD");
        let _ = username;
        let _ = password;
        Box::pin(async { AuthResult::failure(false) })
    }

    fn try_publickey(
        &mut self,
        username: String,
        pubkey: shellfish::identity::Identity,
    ) -> BoxFuture<AuthResult<Self::Identity>> {
        log::info!("TRY PK");
        let _ = username;
        let _ = pubkey;
        Box::pin(async { AuthResult::success(username) })
    }

    fn try_publickey_ok(&mut self, username: String, pubkey: shellfish::identity::Identity) -> BoxFuture<AuthResult<()>> {
        log::info!("TRY PK OK");
        let _ = username;
        let _ = pubkey;
        Box::pin(async { AuthResult::success(()) })
    }
}
