mod process;
mod request;

pub use self::process::*;
pub use self::request::*;

use super::super::error::*;
use super::state::*;
use super::*;

use crate::codec::*;

use futures::task::Poll;

pub struct Session {
    pub channel: Channel<Session>,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.channel.lock().unwrap().terminate_as_user();
    }
}

impl Session {
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
        let mut channel = self.channel.lock().unwrap();
        channel.specific.request = RequestState::Open(request);
        channel.connection_task.wake();
        drop(channel);
        futures::future::poll_fn(|cx| {
            let mut channel = self.channel.lock().unwrap();
            channel.user_task.register(cx.waker());
            match channel.specific.request {
                RequestState::Success => {
                    channel.specific.request = RequestState::None;
                    Poll::Ready(Ok(()))
                }
                RequestState::Failure => {
                    channel.specific.request = RequestState::None;
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
    pub env: Vec<(String, String)>,
    pub stdin: RingBuffer,
    pub stdout: RingBuffer,
    pub stderr: RingBuffer,
    pub request: RequestState<SessionRequest>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            env: Vec::new(),
            stdin: RingBuffer::new(8192),
            stdout: RingBuffer::new(8192),
            stderr: RingBuffer::new(8192),
            request: RequestState::None,
        }
    }
}
