mod process;
mod request;

pub use self::process::*;
pub use self::request::*;

use super::super::error::*;
use super::state::*;
use super::*;

use crate::codec::*;
use crate::ring_buffer::*;

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

/*
impl AsyncWrite for PipeWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut stream = self.0.lock().unwrap();
        stream.writer.register(cx.waker());
        if stream.is_broken() {
            return Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "")));
        }
        if stream.buffer.is_full() {
            stream.reader.wake();
            return Poll::Pending;
        }
        // Do not wake the reader! `poll_flush` does this!
        let written = stream.buffer.write(buf);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        let stream = self.0.lock().unwrap();
        if stream.is_broken() {
            return Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "")));
        }
        if stream.buffer.is_empty() {
            return Poll::Ready(Ok(()))
        }
        stream.writer.register(cx.waker());
        stream.reader.wake();
        Poll::Pending
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Error>> {
        let mut stream = self.0.lock().unwrap();
        stream.close();
        stream.reader.wake();
        Poll::Ready(Ok(()))
    }
}
*/