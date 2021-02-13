pub(crate) mod handle;
pub(crate) mod interconnect;
pub(crate) mod open_failure;
pub(crate) mod open_request;
pub(crate) mod state;
pub(crate) mod types;

pub use self::handle::ChannelHandle;
pub use self::open_failure::ChannelOpenFailure;
pub use self::open_request::ChannelOpenRequest;
pub use self::types::{DirectTcpIp, DirectTcpIpOpen, Session};

use crate::util::codec::{SshDecode, SshEncode};
use std::task::Poll;

pub trait Channel: Unpin + Sized {
    type Open: std::fmt::Debug + Clone + SshEncode + SshDecode + Unpin;
    const NAME: &'static str;
    fn new(channel: ChannelHandle) -> Self;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}
