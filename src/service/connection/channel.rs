mod direct_tcpip;
mod session;

use super::*;

pub use self::direct_tcpip::*;
pub use self::session::*;

pub trait ChannelOpen: Sized {
    type Open: std::fmt::Debug + Clone + Encode + Decode;
    type Confirmation: Encode + Decode;
}

pub (crate) trait Channel: ChannelOpen {
    type Request: ChannelRequest + Encode;
    //type State: ChannelState;

    const NAME: &'static str;

    //fn new_state(max_buffer_size: usize, reply: oneshot::Sender<Result<Self, ChannelOpenFailureReason>>) -> Self::State;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

impl ChannelRequest for () {
    fn name(&self) -> &'static str {
        ""
    }
}

