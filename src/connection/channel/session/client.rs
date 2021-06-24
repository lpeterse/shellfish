use super::super::super::error::ConnectionErrorWatch;
use super::*;
use crate::util::check;

macro_rules! map_err {
    ($x:ident, $res:expr) => {
        match $res {
            Ok(x) => Ok(x),
            Err(_) => Err($x
                .error
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
pub struct Session {
    pub(crate) req_tx: oneshot::Sender<SessionReq>,
    pub(crate) error: ConnectionErrorWatch,
}

impl Session {
    /// Pass an environment variable.
    pub async fn env(&mut self, name: String, value: String) -> Result<(), ConnectionError> {
        let (req_tx, req_rx) = oneshot::channel();
        let (res_tx, res_rx) = oneshot::channel();
        let req_tx = std::mem::replace(&mut self.req_tx, req_tx);
        let req = SessionReq::Env {
            env: (name, value),
            res: res_tx,
            nxt: req_rx,
        };
        map_err!(self, req_tx.send(req))?;
        map_err!(self, res_rx.await)
    }

    /// Request a pseudo-terminal.
    pub async fn pty(&mut self, pty: PtySpecification) -> Result<(), ConnectionError> {
        let (req_tx, req_rx) = oneshot::channel();
        let (res_tx, res_rx) = oneshot::channel();
        let req_tx = std::mem::replace(&mut self.req_tx, req_tx);
        let req = SessionReq::Pty {
            pty,
            res: res_tx,
            nxt: req_rx,
        };
        map_err!(self, req_tx.send(req))?;
        let b = map_err!(self, res_rx.await)?;
        check(b).ok_or(ConnectionError::ChannelPtyRejected)
    }

    /// Execute a remote shell.
    pub async fn shell(self) -> Result<Process, ConnectionError> {
        let (res_tx, res_rx) = oneshot::channel();
        let req = SessionReq::Run {
            res: res_tx,
            run: SessionRun::Shell,
        };
        map_err!(self, self.req_tx.send(req))?;
        map_err!(self, res_rx.await)
    }

    /// Execute a command.
    pub async fn exec(self, command: String) -> Result<Process, ConnectionError> {
        let (res_tx, res_rx) = oneshot::channel();
        let req = SessionReq::Run {
            res: res_tx,
            run: SessionRun::Exec(command),
        };
        map_err!(self, self.req_tx.send(req))?;
        map_err!(self, res_rx.await)
    }

    /// Execute a subsystem.
    pub async fn subsystem(self, subsystem: String) -> Result<Process, ConnectionError> {
        let (res_tx, res_rx) = oneshot::channel();
        let req = SessionReq::Run {
            res: res_tx,
            run: SessionRun::Subsystem(subsystem),
        };
        map_err!(self, self.req_tx.send(req))?;
        map_err!(self, res_rx.await)
    }
}

#[derive(Debug)]
pub(crate) enum SessionReq {
    Env {
        env: (String, String),
        res: oneshot::Sender<()>,
        nxt: oneshot::Receiver<SessionReq>,
    },
    Pty {
        pty: PtySpecification,
        res: oneshot::Sender<bool>,
        nxt: oneshot::Receiver<SessionReq>,
    },
    Run {
        run: SessionRun,
        res: oneshot::Sender<Process>,
    },
}

#[derive(Debug)]
pub enum SessionRun {
    Shell,
    Exec(String),
    Subsystem(String),
}
