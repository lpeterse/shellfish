mod direct_tcpip;
mod session;

use super::*;

pub use self::direct_tcpip::*;
pub use self::session::*;

pub(crate) trait Channel: Sized {
    type Open: Clone + Encode + Decode;
    type Confirmation: Encode + Decode;
    type Request: ChannelRequest + Encode;
    type State: ChannelState;

    const NAME: &'static str;

    fn new_state(local_id: u32, request: &OpenRequest<Self>) -> Self::State;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

impl ChannelRequest for () {
    fn name(&self) -> &'static str {
        ""
    }
}

pub(crate) trait ChannelState: Send {
    fn terminate(&mut self, e: ConnectionError);

    fn local_id(&self) -> u32;
    fn local_window_size(&self) -> u32;
    fn local_max_packet_size(&self) -> u32;
    fn remote_id(&self) -> u32;
    fn remote_window_size(&self) -> u32;
    fn remote_max_packet_size(&self) -> u32;

    fn push_open_confirmation(&mut self, id: u32, ws: u32, ps: u32) -> Result<(), ConnectionError>;
    fn push_open_failure(
        &mut self,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError>;
    fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError>;
    fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError>;
    fn push_eof(&mut self) -> Result<(), ConnectionError>;
    fn push_close(&mut self) -> Result<(), ConnectionError>;
    fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError>;

    fn fail(&mut self) -> Result<(), ConnectionError>;
    fn success(&mut self) -> Result<(), ConnectionError>;
    fn request(&mut self, request: &[u8]) -> Result<(), ConnectionError>;
}
