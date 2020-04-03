use super::*;

use async_std::task::Poll;
use async_std::task::{Context, Waker};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct SessionState(pub(crate) Arc<Mutex<SessionInner>>);

pub(crate) struct SessionInner {
    pub remote_channel: u32,
    pub remote_window_size: u32,
    pub remote_max_packet_size: u32,

    pub is_eof_sent: bool,
    pub is_eof_received: bool,
    pub is_close_sent: bool,
    pub is_close_received: bool,

    pub inner_task: Option<Waker>,
    pub inner_done: Option<ConnectionError>,
    pub outer_task: Option<Waker>,
    pub outer_done: Option<()>,
    pub exit: Option<Exit>,

    pub stdin: Buffer,
    pub stdout: Buffer,
    pub stderr: Buffer,
    pub request: RequestState<SessionRequest>,
}

impl SessionState {
    pub fn set_exit_status(&mut self, status: ExitStatus) {
        //self.exit = Some(Exit::Status(status));
        self.outer_wake()
    }

    pub fn set_exit_signal(&mut self, signal: ExitSignal) {
        //self.exit = Some(Exit::Signal(signal));
        self.outer_wake()
    }

    pub fn inner_wake(&mut self) {
        //self.inner_task.take().map(Waker::wake).unwrap_or(())
    }

    pub fn inner_register(&mut self, cx: &mut Context) {
        //self.inner_task = Some(cx.waker().clone())
    }

    pub fn outer_wake(&mut self) {
        //self.outer_task.take().map(Waker::wake).unwrap_or(())
    }

    pub fn outer_register(&mut self, cx: &mut Context) {
        //self.outer_task = Some(cx.waker().clone())
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(SessionInner {
            remote_channel: 0,
            remote_window_size: 0,
            remote_max_packet_size: 0,
            is_eof_sent: false,
            is_eof_received: false,
            is_close_sent: false,
            is_close_received: false,
            inner_task: None,
            inner_done: None,
            outer_task: None,
            outer_done: None,
            exit: None,
            stdin: Buffer::new(8192),
            stdout: Buffer::new(8192),
            stderr: Buffer::new(8192),
            request: RequestState::None,
        })))
    }
}

impl ChannelState for SessionState {
    fn terminate(&mut self, e: ConnectionError) {}

    fn push_open_confirmation(&mut self, id: u32, ws: u32, ps: u32) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_open_failure(
        &mut self,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_eof(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_close(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        Ok(())
    }

    fn push_failure(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_success(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_request(&mut self, request: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }

    fn poll<T: TransportLayer>(
        &self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        Poll::Pending
    }
}
