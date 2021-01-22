mod handle;
mod handle_inner;
mod interconnect;
mod list;
mod open_future;
mod open_request;
mod state;
mod stream;
mod types;

pub use self::handle::ChannelHandle;
pub use self::handle_inner::ChannelHandleInner;
pub use self::interconnect::Interconnect;
pub use self::open_future::ChannelOpenFuture;
pub use self::open_request::ChannelOpenRequest;
pub use self::types::{DirectTcpIp, DirectTcpIpOpen, Session};

pub(crate) use self::list::ChannelList;

use super::ChannelOpenFailure;
use super::ConnectionError;
use crate::util::codec::{SshDecode, SshEncode};
use crate::util::oneshot;

use std::task::Poll;
use std::sync::Arc;

pub trait Channel: Unpin + Sized {
    type Open: std::fmt::Debug + Clone + SshEncode + SshDecode + Unpin;
    //    type Request: ChannelRequest + SshEncode;

    const NAME: &'static str;

    fn new(channel: ChannelHandle) -> Self;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

pub(crate) type OpenInboundTx = oneshot::Sender<Result<(), ChannelOpenFailure>>;
pub(crate) type OpenInboundRx = oneshot::Receiver<Result<(), ChannelOpenFailure>>;
pub(crate) type OpenOutboundTx =
    oneshot::Sender<Result<Result<ChannelHandle, ChannelOpenFailure>, ConnectionError>>;
pub(crate) type OpenOutboundRx =
    oneshot::Receiver<Result<Result<ChannelHandle, ChannelOpenFailure>, ConnectionError>>;
