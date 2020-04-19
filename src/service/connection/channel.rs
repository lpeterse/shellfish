mod handle;
mod open_future;
mod open_request;
mod slots;
mod state;
mod types;

pub use self::handle::ChannelHandle;
pub use self::open_future::ChannelOpenFuture;
pub use self::open_request::ChannelOpenRequest;
pub use self::types::{DirectTcpIp, DirectTcpIpOpen, Session};

pub(crate) use self::slots::{ChannelSlot, ChannelSlots};

use super::ChannelOpenFailureReason;
use super::ConnectionError;
use crate::codec::{Decode, Encode};
use crate::util::oneshot;

use async_std::task::Poll;
use std::sync::Arc;

pub trait Channel: Unpin + Sized {
    type Open: std::fmt::Debug + Clone + Encode + Decode + Unpin;
    //    type Request: ChannelRequest + Encode;

    const NAME: &'static str;

    fn new(channel: ChannelHandle) -> Self;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

pub(crate) type OpenInboundTx = oneshot::Sender<Result<(), ChannelOpenFailureReason>>;
pub(crate) type OpenInboundRx = oneshot::Receiver<Result<(), ChannelOpenFailureReason>>;
pub(crate) type OpenOutboundTx =
    oneshot::Sender<Result<Result<ChannelHandle, ChannelOpenFailureReason>, ConnectionError>>;
pub(crate) type OpenOutboundRx =
    oneshot::Receiver<Result<Result<ChannelHandle, ChannelOpenFailureReason>, ConnectionError>>;
