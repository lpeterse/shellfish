mod channel;
mod error;
mod future;
mod msg_channel_close;
mod msg_channel_data;
mod msg_channel_eof;
mod msg_channel_extended_data;
mod msg_channel_failure;
mod msg_channel_open;
mod msg_channel_open_confirmation;
mod msg_channel_open_failure;
mod msg_channel_request;
mod msg_channel_success;
mod msg_channel_window_adjust;
mod msg_global_request;
mod request;

use super::*;

pub use self::channel::*;
pub use self::error::*;
use self::request::*;
use self::future::ConnectionFuture;
use self::msg_channel_open_failure::ChannelOpenFailureReason;

use crate::codec::*;
use crate::transport::*;
use crate::role::*;
use crate::client::*;

use futures::channel::oneshot;
use futures::future::FutureExt;

pub struct Connection<R: Role> {
    phantom: std::marker::PhantomData<R>,
    error: oneshot::Receiver<ConnectionError>,
    request_sender: RequestSender,
    request_receiver: RequestReceiver,
}

impl <R: Role> Service for Connection<R> {
    const NAME: &'static str = "ssh-connection";
}

impl <R: Role> Connection<R> {
    pub fn new<T: TransportStream>(t: Transport<R, T>) -> Connection<R> {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = channel();
        let (s3, r3) = channel();
        let future = ConnectionFuture::new(t, s3, r2).map(|e| {
            log::warn!("Connection died with {:?}", e);
            s1.send(e.unwrap_err()).unwrap_or(())
        });
        async_std::task::spawn(future);
        Connection {
            phantom: std::marker::PhantomData,
            error: r1,
            request_sender: s2,
            request_receiver: r3,
        }
    }

    pub async fn disconnect(mut self) {
        self.request_sender
            .request(DisconnectRequest {})
            .await
            .unwrap_or(())
    }
}

impl Connection<Client> {
    pub async fn session(
        &mut self,
    ) -> Result<Result<Session, ChannelOpenFailure>, ConnectionError> {
        self.request_sender
            .request(ChannelOpenRequest {
                initial_window_size: 8192,
                max_packet_size: 1024,
            })
            .await
    }
}
