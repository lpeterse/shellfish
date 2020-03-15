mod open;

use super::*;

pub(crate) use self::open::*;

use std::sync::{Arc, Mutex};

pub struct DirectTcpIp(Arc<Mutex<DirectTcpIpState>>);

impl Channel for DirectTcpIp {
    type Open = DirectTcpIpOpen;
    type Confirmation = ();
    type Request = ();
    type State = DirectTcpIpState;
    const NAME: &'static str = "direct-tcpip";

    fn new_state(local_id: u32, req: &OpenRequest<Self>) -> Self::State {
        DirectTcpIpState { }
    }
}

#[derive(Debug, Default)]
pub(crate) struct DirectTcpIpState {}

impl ChannelState for DirectTcpIpState {
    fn terminate(&mut self, e: ConnectionError) {
        todo!()
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
