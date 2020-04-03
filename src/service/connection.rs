mod channel;
mod channels;
mod config;
mod error;
mod future;
mod msg;
mod request;

pub use self::config::*;
pub use self::error::*;

use self::channel::*;
use self::channels::*;
use self::future::ConnectionFuture;
use self::msg::*;
use self::request::*;
use super::*;

use crate::client::*;
use crate::codec::*;
use crate::role::*;
use crate::transport::{DisconnectReason, Socket, Transport, TransportLayer, TransportLayerExt};
use crate::util::oneshot;

use async_std::future::Future;
use async_std::task::ready;
use std::pin::Pin;
use std::task::{Context, Poll};

/// The connection protocol offers channel multiplexing for a variety of applications like remote
/// shell and command execution as well as TCP/IP port forwarding and various other extensions.
///
/// Unless client or server request a service on top of this protocol the connection just keeps
/// itself alive and does nothing. Dropping the connection object will close the connection and
/// free all resources. It will also terminate all dependant channels (shells and forwardings etc).
pub struct Connection<R: Role> {
    phantom: std::marker::PhantomData<R>,
    close: oneshot::Sender<DisconnectReason>,
    error: oneshot::Receiver<ConnectionError>,
    requests: RequestSender,
}

impl<R: Role> Service<R> for Connection<R>
where
    R::Config: ConnectionConfig,
{
    const NAME: &'static str = "ssh-connection";

    /// Create a new connection.
    ///
    /// The connection spawns a separate handler thread. This handler thread's lifetime is linked
    /// the `Connection` object: `Drop`ping the connection will send it a termination signal.
    fn new<T: TransportLayer>(config: &R::Config, transport: T) -> Connection<R> {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = oneshot::channel();
        let (s3, r3) = channel();
        let (s4, r4) = channel();
        let future = ConnectionFuture::new(config, transport, r1, r3, s4);
        async_std::task::spawn(async { s2.send(future.await) });
        Connection {
            phantom: std::marker::PhantomData,
            close: s1,
            error: r2,
            requests: s3,
        }
    }
}

impl Connection<Client> {
    /// Request the connection service.
    ///
    /// This method consumes a `Transport` object and requests the `ssh-connection` protocol.
    /// Upon server confirmation it returns a protocol specific `Connection` handle which offers
    /// all service specific operations.
    pub async fn request<S: Socket>(
        transport: Transport<Client, S>,
        config: &ClientConfig,
    ) -> Result<Self, ConnectionError> {
        let transport = TransportLayerExt::request_service(transport, Self::NAME).await?;
        Ok(<Self as Service<Client>>::new(config, transport))
    }

    /// Request a new session on top of an established connection.
    ///
    /// A connection is able to multiplex several sessions simultaneously so this method may be
    /// called more than once on a given connection. This method may fail if either the client
    /// (due to config limitiation) or the server hits a limit on the number of concurrent
    /// channels per connection.
    pub async fn session(
        &mut self,
    ) -> Result<Result<Session, ChannelOpenFailureReason>, ConnectionError> {
        let req: OpenRequest<Session> = OpenRequest { specific: () };
        self.requests.request(req).await
    }

    pub async fn direct_tcpip(
        &mut self,
        dst_host: String,
        dst_port: u32,
        src_addr: String,
        src_port: u32,
    ) -> Result<Result<DirectTcpIp, ChannelOpenFailureReason>, ConnectionError> {
        let req: OpenRequest<DirectTcpIp> = OpenRequest {
            specific: DirectTcpIpOpen {
                dst_host,
                dst_port,
                src_addr,
                src_port,
            },
        };
        self.requests.request(req).await
    }
}

impl<R: Role> Drop for Connection<R> {
    fn drop(&mut self) {
        let x = std::mem::replace(&mut self.close, oneshot::channel().0);
        x.send(DisconnectReason::BY_APPLICATION);
    }
}

impl Future for Connection<Client> {
    type Output = ConnectionError;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = Pin::into_inner(self);
        let r = ready!(Pin::new(&mut s.error).poll(cx));
        Poll::Ready(r.unwrap_or(ConnectionError::Terminated))
    }
}
