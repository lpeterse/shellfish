mod channel;
mod error;
mod future;
mod msg_channel_eof;
mod msg_channel_close;
mod msg_channel_data;
mod msg_channel_extended_data;
mod msg_channel_failure;
mod msg_channel_open;
mod msg_channel_open_confirmation;
mod msg_channel_open_failure;
mod msg_channel_request;
mod msg_channel_success;
mod msg_channel_window_adjust;
mod msg_global_request;

use super::*;

pub use self::channel::*;
pub use self::error::*;
use self::future::ConnectionFuture;
use self::msg_channel_open_failure::ChannelOpenFailureReason;

use crate::codec::*;
use crate::requestable;
use crate::transport::*;

use futures::channel::oneshot;
use futures::future::FutureExt;
use std::convert::{TryFrom};

pub struct Connection {
    error: oneshot::Receiver<ConnectionError>,
    request_sender: requestable::Sender<Connection>,
    request_receiver: requestable::Receiver<Connection>,
}

impl Service for Connection {
    const NAME: &'static str = "ssh-connection";
}

impl Connection {
    pub fn new<T: TransportStream>(t: Transport<T>) -> Connection {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = requestable::channel(32);
        let (s3, r3) = requestable::channel(32);
        let future = ConnectionFuture::new(t, s3, r2).map(|e| {
            log::warn!("Connection died with {:?}", e);
            s1.send(e).unwrap_or(())
        });
        async_std::task::spawn(future);
        Connection {
            error: r1,
            request_sender: s2,
            request_receiver: r3,
        }
    }

    pub async fn disconnect(mut self) {
        self.request_sender
            .request(ConnectionRequest::Disconnect)
            .await
            .unwrap_or(())
    }

    pub async fn debug(&mut self, msg: String) -> Result<(), ConnectionError> {
        self.request_sender
            .request(ConnectionRequest::Debug(msg))
            .await
    }

    pub async fn session(
        &mut self,
    ) -> Result<Result<Session, ChannelOpenFailureReason>, ConnectionError> {
        self.request_sender
            .request(ConnectionRequest::ChannelOpen(ChannelOpenRequest {
                initial_window_size: 1024,
                max_packet_size: 1024,
            }))
            .await
    }
}

impl requestable::Requestable for Connection {
    type Request = ConnectionRequest;
    type Response = ConnectionResponse;
    type Error = ConnectionError;
}

pub enum ConnectionRequest {
    Disconnect,
    Debug(String),
    ChannelOpen(ChannelOpenRequest),
}

pub struct ChannelOpenRequest {
    initial_window_size: u32,
    max_packet_size: u32,
}

pub enum ConnectionResponse {
    Ok,
    OpenSession(Session),
    OpenFailure(ChannelOpenFailureReason),
}

impl TryFrom<ConnectionResponse> for () {
    type Error = ();
    fn try_from(x: ConnectionResponse) -> Result<Self, ()> {
        match x {
            ConnectionResponse::Ok => Ok(()),
            _ => Err(()),
        }
    }
}

impl TryFrom<ConnectionResponse> for Session {
    type Error = ();
    fn try_from(x: ConnectionResponse) -> Result<Self, ()> {
        match x {
            ConnectionResponse::OpenSession(c) => Ok(c),
            _ => Err(()),
        }
    }
}

impl TryFrom<ConnectionResponse> for Result<Session, ChannelOpenFailureReason> {
    type Error = ();
    fn try_from(x: ConnectionResponse) -> Result<Self, ()> {
        match x {
            ConnectionResponse::OpenFailure(reason) => Ok(Err(reason)),
            _ => match TryFrom::try_from(x) {
                Ok(t) => Ok(Ok(t)),
                Err(_) => Err(()),
            },
        }
    }
}

impl From<ChannelOpenFailureReason> for ConnectionResponse {
    fn from(x: ChannelOpenFailureReason) -> Self {
        ConnectionResponse::OpenFailure(x)
    }
}

impl TryFrom<ConnectionRequest> for ChannelOpenRequest {
    type Error = ();
    fn try_from(x: ConnectionRequest) -> Result<ChannelOpenRequest, ()> {
        match x {
            ConnectionRequest::ChannelOpen(x) => Ok(x),
            _ => Err(()),
        }
    }
}
