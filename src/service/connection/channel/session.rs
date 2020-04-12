mod exit;
mod process;
mod request;
mod signal;

pub(crate) use self::exit::*;
pub(crate) use self::process::*;
pub(crate) use self::request::*;
pub(crate) use self::signal::*;

use super::super::*;
use super::*;
use crate::client::Client;

use crate::codec::*;

/// A session is a remote execution of a program.  The program may be a
/// shell, an application, a system command, or some built-in subsystem.
/// It may or may not have a tty, and may or may not involve X11
/// forwarding.  Multiple sessions can be active simultaneously.
#[derive(Debug)]
pub struct Session<R: Role> {
    role: std::marker::PhantomData<R>,
    channel: ChannelState,
}

impl<R: Role> Session<R> {
    pub(crate) fn new(channel: ChannelState) -> Self {
        Self {
            role: Default::default(),
            channel,
        }
    }
}

impl Session<Client> {
    /// Execute a remote shell.
    pub async fn shell(self) -> Result<Process<Client>, ConnectionError> {
        let req = SessionRequest::ShellRequest(ShellRequest {});
        Ok(Process::<Client>(self.request(req).await?))
    }

    /// Execute a command.
    pub async fn exec(self, command: String) -> Result<Process<Client>, ConnectionError> {
        let req = SessionRequest::ExecRequest(ExecRequest { command });
        Ok(Process::<Client>(self.request(req).await?))
    }

    /// Execute a subsystem.
    pub async fn subsystem(self, subsystem: String) -> Result<Process<Client>, ConnectionError> {
        let req = SessionRequest::SubsystemRequest(SubsystemRequest { subsystem });
        Ok(Process::<Client>(self.request(req).await?))
    }

    pub async fn request_env(&self) -> Result<(), ConnectionError> {
        Ok(())
    } 

    async fn request(self, request: SessionRequest) -> Result<Self, ConnectionError> {
        /*
        let mut state = (self.state)
            .0
            .lock()
            .map_err(|_| ConnectionError::Unknown)?;
        state.request = RequestState::Open(request);
        //state.inner_wake(); // FIXME
        drop(state);
        async_std::future::poll_fn(|cx| {
            let mut state = (self.state)
                .0
                .lock()
                .map_err(|_| ConnectionError::Unknown)?;
            match state.request {
                RequestState::Success => {
                    state.request = RequestState::None;
                    Poll::Ready(Ok(()))
                }
                RequestState::Failure => {
                    state.request = RequestState::None;
                    Poll::Ready(Err(ConnectionError::ChannelRequestFailure))
                }
                _ => {
                    // state.outer_register(cx); // FIXME
                    Poll::Pending
                }
            }
        })
        .await?;
        Ok(self)
        */
        todo!()
    }
}

impl<R: Role> ChannelOpen for Session<R> {
    type Open = ();
    type Confirmation = ();
}

impl<R: Role> Channel for Session<R> {
    type Request = SessionRequest;

    const NAME: &'static str = "session";
}

impl<R: Role> Drop for Session<R> {
    fn drop(&mut self) {
        let mut x = self.channel.0.lock().unwrap();
        x.close_tx = Some(false);
        x.wake_inner_task();
    }
}
