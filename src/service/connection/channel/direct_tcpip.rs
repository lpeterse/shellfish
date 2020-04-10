mod open;

use super::*;

pub(crate) use self::open::*;

use crate::buffer::*;

use async_std::task::Poll;
use async_std::task::Waker;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct DirectTcpIp(Arc<Mutex<DirectTcpIpState>>);

impl ChannelOpen for DirectTcpIp {
    type Open = DirectTcpIpOpen;
    type Confirmation = ();
}

impl Channel for DirectTcpIp {
    type Request = ();
    type State = DirectTcpIpState;

    const NAME: &'static str = "direct-tcpip";

    fn new_state(max_buffer_size: usize, reply: oneshot::Sender<Result<Self, ChannelOpenFailureReason>>) -> Self::State {
        Self::State {
            local_window_size: max_buffer_size as u32,
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

            data_in: Buffer::new(max_buffer_size),
            data_out: Buffer::new(max_buffer_size),
        }
    }
}

#[derive(Debug)]
pub(crate) struct DirectTcpIpState {
    local_window_size: u32,

    remote_channel: u32,
    remote_window_size: u32,
    remote_max_packet_size: u32,

    is_eof_sent: bool,
    is_eof_received: bool,
    is_close_sent: bool,
    is_close_received: bool,

    inner_task: Option<Waker>,
    inner_done: Option<ConnectionError>,
    outer_task: Option<Waker>,
    outer_done: Option<()>,

    data_in: Buffer,
    data_out: Buffer,
}

impl ChannelState for DirectTcpIpState {
    fn push_open_confirmation(&mut self, ch: u32, ws: u32, ps: u32) -> Result<(), ConnectionError> {
        self.remote_channel = ch;
        self.remote_window_size = ws;
        self.remote_max_packet_size = ps;
        /// FIXME wake
        Ok(())
    }
    fn push_open_failure(
        &mut self,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError> {
        /// FIXME wake
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

    fn push_request(&mut self, request: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_success(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    fn push_failure(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }

    fn terminate(&mut self, e: ConnectionError) {
        todo!()
    }

    fn poll<T: TransportLayer>(
        &self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        Poll::Pending
    }
}
