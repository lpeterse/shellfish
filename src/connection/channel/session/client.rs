use super::super::super::error::*;
use super::super::Channel;
use super::*;
use crate::connection::channel::RequestFailure;
use crate::connection::{channel::ChannelState, ConnectionConfig, OpenFailure};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::watch;

macro_rules! map_err {
    ($x:ident, $res:expr) => {
        match $res {
            Ok(x) => Ok(x),
            Err(_) => Err($x
                .err_rx
                .borrow()
                .as_deref()
                .map(|x| x.clone())
                .unwrap_or(ConnectionError::Dropped)),
        }
    };
}

/// A session is a remote execution of a program.  The program may be a
/// shell, an application, a system command, or some built-in subsystem.
/// It may or may not have a tty, and may or may not involve X11
/// forwarding.  Multiple sessions can be active simultaneously.
#[derive(Debug)]
pub struct SessionClient {
    req_tx: R1,
    err_rx: ConnectionErrorWatch,
}

impl SessionClient {
    pub(crate) fn new(req_tx: R1, err_rx: ConnectionErrorWatch) -> Self {
        Self { req_tx, err_rx }
    }

    pub(crate) fn open(
        config: &ConnectionConfig,
        lid: u32,
        error_rx: watch::Receiver<Option<Arc<ConnectionError>>>,
        reply_tx: oneshot::Sender<Result<SessionClient, OpenFailure>>,
    ) -> Box<dyn ChannelState> {
        let lbs = config.channel_max_buffer_size;
        let lps = config.channel_max_packet_size;
        let cst = ClientState1::new(lid, lbs, lps, error_rx, reply_tx);
        Box::new(cst)
    }

    /// Pass an environment variable.
    pub async fn env(
        &mut self,
        name: &str,
        value: &str,
        want_reply: bool,
    ) -> Result<Result<(), RequestFailure>, ConnectionError> {
        let param = SessionReq2::Env((name.into(), value.into()));
        let response = self.req_tx.req_unit(param, want_reply);
        map_err!(self, response.await)
    }

    /// Request a pseudo-terminal.
    pub async fn pty(
        &mut self,
        pty: &PtySpecification,
        want_reply: bool,
    ) -> Result<Result<(), RequestFailure>, ConnectionError> {
        let param = SessionReq2::Pty(pty.clone());
        let response = self.req_tx.req_unit(param, want_reply);
        map_err!(self, response.await)
    }

    /// Execute a remote shell.
    pub async fn shell(self) -> Result<Result<Box<dyn Process>, RequestFailure<Self>>, ConnectionError> {
        let mut s = self;
        let param = SessionReq2::Shell;
        let response = s.req_tx.req_proc(param);
        Ok(map_err!(s, response.await)?.map_err(|_| RequestFailure(s)))
    }

    /// Execute a command.
    pub async fn exec(self, command: &str) -> Result<Result<Box<dyn Process>, RequestFailure<Self>>, ConnectionError> {
        let mut s = self;
        let param = SessionReq2::Exec(command.into());
        let response = s.req_tx.req_proc(param);
        Ok(map_err!(s, response.await)?.map_err(|_| RequestFailure(s)))
    }

    /// Execute a subsystem.
    pub async fn subsystem(self, subsystem: &str) -> Result<Result<Box<dyn Process>, RequestFailure<Self>>, ConnectionError> {
        let mut s = self;
        let param = SessionReq2::Subsystem(subsystem.into());
        let response = s.req_tx.req_proc(param);
        Ok(map_err!(s, response.await)?.map_err(|_| RequestFailure(s)))
    }
}

impl Channel for SessionClient {
    const NAME: &'static str = "session";
}
