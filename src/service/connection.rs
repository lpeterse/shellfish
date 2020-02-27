mod channel;
mod config;
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
mod msg_request_failure;
mod msg_request_success;
mod request;

pub use self::channel::*;
pub use self::config::*;
pub use self::error::*;

use self::future::ConnectionFuture;
use self::msg_channel_open_failure::Reason;
use self::request::*;
use super::*;

use crate::client::*;
use crate::codec::*;
use crate::role::*;
use crate::transport::*;

use futures::channel::oneshot;
use futures::future::{Future, FutureExt};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Connection<R: Role> {
    phantom: std::marker::PhantomData<R>,
    request_sender: RequestSender,
    request_receiver: RequestReceiver,
    result: oneshot::Receiver<Result<(), ConnectionError>>,
}

impl<R: Role> Service<R> for Connection<R>
where
    R::Config: ConnectionConfig,
{
    const NAME: &'static str = "ssh-connection";

    fn new<S: Socket>(config: &R::Config, transport: Transport<R, S>) -> Connection<R> {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = channel();
        let (s3, r3) = channel();
        async_std::task::spawn(
            ConnectionFuture::new(config, transport, s3, r2).map(|r| s1.send(r).unwrap_or(())),
        );
        Connection {
            phantom: std::marker::PhantomData,
            request_sender: s2,
            request_receiver: r3,
            result: r1,
        }
    }
}

impl<R: Role> Connection<R> {
    pub async fn disconnect(mut self) {
        self.request_sender
            .request(DisconnectRequest {})
            .await
            .unwrap_or(())
    }
}

impl Connection<Client> {
    pub async fn request<S: Socket>(
        transport: Transport<Client, S>,
        config: &ClientConfig,
    ) -> Result<Self, ConnectionError> {
        let transport = transport.request_service(Self::NAME).await?;
        Ok(<Self as Service<Client>>::new(config, transport))
    }

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

impl<R: Role> Future for Connection<R> {
    type Output = Result<(), ConnectionError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let x = Pin::into_inner(self);
        Pin::new(&mut x.result).poll(cx).map(|r| match r {
            Err(_) => Err(ConnectionError::Terminated),
            Ok(r) => r,
        })
    }
}
