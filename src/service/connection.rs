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
mod msg_request_success;
mod msg_request_failure;
mod request;

pub use self::channel::*;
pub use self::error::*;
pub use self::config::*;

use super::*;
use self::future::ConnectionFuture;
use self::msg_channel_open_failure::Reason;
use self::request::*;

use crate::client::*;
use crate::codec::*;
use crate::role::*;
use crate::transport::*;

use async_std::net::TcpStream;
use futures::channel::oneshot;
use futures::future::FutureExt;

pub struct Connection<R: Role> {
    phantom: std::marker::PhantomData<R>,
    error: oneshot::Receiver<ConnectionError>,
    request_sender: RequestSender,
    request_receiver: RequestReceiver,
}

impl<R: Role> Service<R> for Connection<R>
where
    R::Config: ConnectionConfig
{
    const NAME: &'static str = "ssh-connection";

    fn new(config: &R::Config, transport: Transport<R, TcpStream>) -> Connection<R> {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = channel();
        let (s3, r3) = channel();
        let future = ConnectionFuture::new(config, transport, s3, r2).map(|e| {
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
