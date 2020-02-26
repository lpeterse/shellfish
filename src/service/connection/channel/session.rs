mod exit;
mod process;
mod request;

pub use self::exit::*;
pub use self::process::*;
pub use self::request::*;

use super::*;

use crate::codec::*;
use crate::buffer::*;

use futures::task::AtomicWaker;
use futures::task::Poll;
use std::sync::{Arc, Mutex};

pub struct Session {
    state: Arc<Mutex<SessionState>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.outer_error = Some(());
        state.inner_waker.wake();
    }
}

impl Session {
    pub(crate) fn new(state: Arc<Mutex<SessionState>>) -> Self {
        Self { state }
    }

    pub async fn exec(mut self, command: String) -> Result<Process, ConnectionError> {
        self.request(SessionRequest::ExecRequest(ExecRequest { command }))
            .await?;
        Ok(Process::new(self))
    }

    pub async fn shell(mut self) -> Result<Process, ConnectionError> {
        self.request(SessionRequest::ShellRequest(ShellRequest {}))
            .await?;
        Ok(Process::new(self))
    }

    pub async fn subsystem(mut self, subsystem: String) -> Result<Process, ConnectionError> {
        self.request(SessionRequest::SubsystemRequest(SubsystemRequest {
            subsystem,
        }))
        .await?;
        Ok(Process::new(self))
    }

    async fn request(&mut self, request: SessionRequest) -> Result<(), ConnectionError> {
        let mut state = self.state.lock().unwrap();
        state.request = RequestState::Open(request);
        state.inner_waker.wake();
        drop(state);
        futures::future::poll_fn(|cx| {
            let mut state = self.state.lock().unwrap();
            state.outer_waker.register(cx.waker());
            match state.request {
                RequestState::Success => {
                    state.request = RequestState::None;
                    Poll::Ready(Ok(()))
                }
                RequestState::Failure => {
                    state.request = RequestState::None;
                    Poll::Ready(Err(ConnectionError::ChannelRequestFailure))
                }
                _ => Poll::Pending,
            }
        })
        .await
    }
}

impl ChannelType for Session {
    type Open = ();
    type Confirmation = ();
    type Request = SessionRequest;
    type SpecificState = SessionState;

    const NAME: &'static str = "session";
}

pub struct SessionState {
    pub is_closed: bool,
    pub is_local_eof: bool,
    pub is_remote_eof: bool,
    pub inner_waker: AtomicWaker,
    pub inner_error: Option<ConnectionError>,
    pub outer_waker: AtomicWaker,
    pub outer_error: Option<()>,
    pub env: Vec<(String, String)>,
    pub exit: Option<Exit>,
    pub stdin: Buffer,
    pub stdout: Buffer,
    pub stderr: Buffer,
    pub request: RequestState<SessionRequest>,
}

impl SessionState {
    pub fn add_env(&mut self, env: (String, String)) {
        self.env.push(env);
        self.outer_waker.wake();
    }

    pub fn set_exit_status(&mut self, status: ExitStatus) {
        self.exit = Some(Exit::Status(status));
        self.outer_waker.wake();
    }

    pub fn set_exit_signal(&mut self, signal: ExitSignal) {
        self.exit = Some(Exit::Signal(signal));
        self.outer_waker.wake();
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            is_closed: false,
            is_local_eof: false,
            is_remote_eof: false,
            inner_waker: AtomicWaker::new(),
            inner_error: None,
            outer_waker: AtomicWaker::new(),
            outer_error: None,
            env: Vec::new(),
            exit: None,
            stdin: Buffer::new(8192),
            stdout: Buffer::new(8192),
            stderr: Buffer::new(8192),
            request: RequestState::None,
        }
    }
}

impl SpecificState for SessionState {
    fn terminate(&mut self, e: ConnectionError) {
        self.inner_error = Some(e);
        self.outer_waker.wake();
    }
}
