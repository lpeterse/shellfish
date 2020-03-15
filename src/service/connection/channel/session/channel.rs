use super::*;

use async_std::task::{Context, Waker};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct SessionState(pub(crate) Arc<Mutex<SessionInner>>);

pub(crate) struct SessionInner {
    pub is_closing: bool,
    pub local_channel: u32,
    pub local_window_size: u32,
    pub local_max_packet_size: u32,
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
    pub fn new(
        local_channel: u32,
        local_window_size: u32,
        local_max_packet_size: u32,
    ) -> Self {
        Self(Arc::new(Mutex::new(SessionInner {
            is_closing: false,
            local_channel,
            local_window_size,
            local_max_packet_size,
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

/*
impl ChannelState for SessionChannel {
    fn terminate(&mut self, e: ConnectionError) {
        self.inner_done = Some(e);
        self.outer_wake();
    }
}*/

impl ChannelState for SessionState {
    fn terminate(&mut self, e: ConnectionError) {

    }

    fn local_id(&self) -> u32 {
        0
    }
    fn local_window_size(&self) -> u32 {
        0
    }
    fn local_max_packet_size(&self) -> u32 {
        0
    }
    fn remote_id(&self) -> u32 {
        0
    }
    fn remote_window_size(&self) -> u32 {
        0
    }
    fn remote_max_packet_size(&self) -> u32 {
        0
    }

    fn push_open_confirmation(&mut self, id: u32, ws: u32, ps: u32) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_open_failure(&mut self, reason: ChannelOpenFailureReason) -> Result<(), ConnectionError> {
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

    fn fail(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn success(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn request(&mut self, request: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }

}

/*
impl Channel {
    pub fn new(
        local_channel: u32,
        local_window_size: u32,
        local_max_packet_size: u32,
        remote_channel: u32,
        remote_window_size: u32,
        remote_max_packet_size: u32,
        shared: SharedState,
    ) -> Self {
        Self {
            is_closing: false,
            local_channel,
            local_window_size,
            local_max_packet_size,
            remote_channel,
            remote_window_size,
            remote_max_packet_size,
            shared,
        }
    }

    pub fn is_closing(&self) -> bool {
        self.is_closing
    }

    pub fn local_channel(&self) -> u32 {
        self.local_channel
    }

    pub fn remote_channel(&self) -> u32 {
        self.remote_channel
    }

    pub fn shared(&self) -> &SharedState {
        &self.shared
    }

    pub fn decrease_local_window_size(&mut self, n: u32) -> Result<(), ConnectionError> {
        if n <= self.local_window_size {
            self.local_window_size -= n;
            return Ok(());
        }
        Err(ConnectionError::ChannelWindowSizeUnderflow)
    }

    pub fn increase_remote_window_size(&mut self, n: u32) -> Result<(), ConnectionError> {
        let n_: u64 = n as u64;
        let w_: u64 = self.remote_window_size as u64;
        if n_ + w_ <= (u32::max_value() as u64) {
            self.remote_window_size += n;
            return Ok(());
        }
        Err(ConnectionError::ChannelWindowSizeOverflow)
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        match &mut self.shared {
            SharedState::Session(x) => x.lock().unwrap().terminate(e),
        }
    }
}
*/
