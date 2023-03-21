use shellfish::agent::*;
use shellfish::connection::{ConnectionHandler, SessionRequest, DirectTcpIpRequest, GlobalRequestWantReply, ConnectionError, GlobalRequest, SessionHandler, Process, Exit, ExitStatus, ExitSignal};
use shellfish::server::*;
use shellfish::user_auth::{AuthResult, UserAuthSession};
use shellfish::util::process::ChildProcess;
use shellfish::util::{BoxFuture, check};
use shellfish::util::pty::Pty;
use tokio::io::{ReadBuf, AsyncRead, AsyncWrite};
use std::ffi::{OsStr, CStr};
use std::fs::File;
use std::future::Future;
use std::os::unix::prelude::{AsRawFd, OsStrExt, AsFd, FromRawFd};
use std::pin::Pin;
use tokio::pin;
use std::process::{Stdio};
use tokio::task::JoinSet;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use tokio::process::{Command, Child};
use std::collections::HashMap;
use std::os::unix::prelude::OwnedFd;

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

#[derive(Debug)]
pub struct MyIdentity {
    username: String,
}

impl MyIdentity {
    pub fn new(username: String) -> Self {
        Self { username }
    }
}

struct MyServerHandler {}

impl MyServerHandler {
    fn new() -> Self {
        Self {}
    }
}

impl ServerHandler for MyServerHandler {
    type Identity = MyIdentity;

    fn on_accept(
        &self,
        addr: SocketAddr,
    ) -> BoxFuture<Option<Box<dyn UserAuthSession<Identity = Self::Identity>>>> {
        log::info!("New connection: {}", addr);
        let x: Box<dyn UserAuthSession<Identity = MyIdentity>> = Box::new(MyUserAuthProvider);
        Box::pin(async { Some(x) })
    }

    fn on_error(&self, err: ServerError) {
        log::error!("{}", err);
    }

    fn on_authenticated(&self, identity: Self::Identity) -> BoxFuture<Box<dyn ConnectionHandler>> {
        Box::pin(async {
            let x: Box<dyn ConnectionHandler> = Box::new(MyConnectionHandler::new(identity));
            x
        })
    }
}

#[derive(Debug)]
pub struct MyUserAuthProvider;

impl UserAuthSession for MyUserAuthProvider {
    type Identity = MyIdentity;

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
        Box::pin(async { AuthResult::success(MyIdentity::new(username)) })
    }

    fn try_publickey_ok(
        &mut self,
        username: String,
        pubkey: shellfish::identity::Identity,
    ) -> BoxFuture<AuthResult<()>> {
        log::info!("TRY PK OK");
        let _ = username;
        let _ = pubkey;
        Box::pin(async { AuthResult::success(()) })
    }
}

#[derive(Debug)]
pub struct MyConnectionHandler {
    identity: MyIdentity,
    subtasks: JoinSet<()>
}

impl MyConnectionHandler {
    pub fn new(identity: MyIdentity) -> Self {
        Self { identity, subtasks: JoinSet::new() }
    }
}

impl ConnectionHandler for MyConnectionHandler {
    fn on_request(&mut self, request: GlobalRequest) {
        log::error!("HANDLER ON_REQUEST: {:?}", request);
    }

    fn on_request_want_reply(&mut self, request: GlobalRequestWantReply) {
        log::error!("HANDLER ON_REQUEST_WANT_REPLY {:?}", request);
    }

    fn on_direct_tcpip_request(&mut self, request: DirectTcpIpRequest) {
        log::error!("HANDLER ON_DIRECT_TCPIP_REQUEST {:?}", request);
    }

    fn on_session_request(&mut self, request: SessionRequest) {
        log::error!("HANDLER ON_SESSION_REQUEST {:?}", request);
        let handler = Box::new(MySessionHandler::default());
        request.accept(handler);
    }

    fn on_error(self: Box<Self>, e: &ConnectionError) {
        log::error!("HANDLER ON_ERROR: {}", e);
    }

    fn poll(&mut self, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }
}

#[derive(Debug, Default)]
pub struct MySessionHandler {
    pty: Option<Pty>,
    envs: HashMap<String, String>,
}

impl SessionHandler for MySessionHandler {
    fn on_env_request(&mut self, key: String, val: String) -> Option<()> {
        log::error!("ENV {}={}", key, val);
        let _ = self.envs.insert(key, val);
        Some(())
    }

    fn on_pty_request(&mut self) -> Option<()> {
        check(self.pty.is_none())?;
        self.pty = Pty::new();
        self.pty.as_ref().map(drop)
    }

    fn on_shell_request(self: Box<Self>) -> Option<Box<dyn Process>> {
        let cmd = Command::new("/bin/bash");
        let proc = ChildProcess::spawn(cmd, self.pty).ok()?;
        Some(Box::new(proc))
    }

    fn on_exec_request(self: Box<Self>, cmd: &str) -> Option<Box<dyn Process>> {
        let cmd = Command::new(cmd);
        let proc = ChildProcess::spawn(cmd, self.pty).ok()?;
        Some(Box::new(proc))
    }

    fn on_subsystem_request(self: Box<Self>, subsystem: &str) -> Option<Box<dyn Process>>{
        todo!()
    }
}
