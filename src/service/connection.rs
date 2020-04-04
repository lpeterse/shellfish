mod channel;
mod channels;
mod config;
mod error;
mod future;
mod global;
mod msg;
mod request;

pub use self::config::*;
pub use self::error::*;
pub use self::global::*;

use self::channel::*;
use self::channels::*;
use self::future::ConnectionFuture;
use self::msg::*;
use self::request::*;
use super::*;

use crate::client::Client;
use crate::codec::*;
use crate::role::*;
use crate::server::Server;
use crate::transport::{DisconnectReason, TransportLayer, TransportLayerExt};
use crate::util::oneshot;

use async_std::future::Future;
use async_std::stream::Stream;
use async_std::sync;
use async_std::task::ready;
use std::pin::Pin;
use std::task::{Context, Poll};

/// The connection protocol offers channel multiplexing for a variety of applications like remote
/// shell and command execution as well as TCP/IP port forwarding and various other extensions.
///
/// Unless client or server request a service on top of this protocol the connection just keeps
/// itself alive and does nothing. Dropping the connection object will close the connection and
/// free all resources. It will also terminate all dependant channels (shells and forwardings etc).
pub struct Connection {
    close: oneshot::Sender<DisconnectReason>,
    error: oneshot::Receiver<ConnectionError>,
    requests: RequestSender,
    requests_rx: sync::Receiver<ConnectionRequest>,
}

impl<R: Role> Service<R> for Connection
where
    R::Config: ConnectionConfig,
{
    const NAME: &'static str = "ssh-connection";

    fn new<T: TransportLayer>(config: &R::Config, transport: T) -> Self {
        Self::new(config, transport)
    }
}

impl Connection {
    /// Create a new connection.
    ///
    /// The connection spawns a separate handler thread. This handler thread's lifetime is linked
    /// the `Connection` object: `Drop`ping the connection will send it a termination signal.
    fn new<C: ConnectionConfig, T: TransportLayer>(config: &C, transport: T) -> Connection {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = oneshot::channel();
        let (s3, r3) = channel();
        let (s4, r4) = channel();
        let (_, requests_rx) = sync::channel(2);
        let future = ConnectionFuture::new(config, transport, r1, r3, s4);
        async_std::task::spawn(async { s2.send(future.await) });
        Connection {
            close: s1,
            error: r2,
            requests: s3,
            requests_rx,
        }
    }

    /// Request the connection service.
    ///
    /// This method consumes a `Transport` object and requests the `ssh-connection` protocol.
    /// Upon server confirmation it returns a protocol specific `Connection` handle which offers
    /// all service specific operations.
    pub async fn request<C: ConnectionConfig, T: TransportLayer>(
        transport: T,
        config: &C,
    ) -> Result<Self, ConnectionError> {
        let transport =
            TransportLayerExt::request_service(transport, <Self as Service<Client>>::NAME).await?;
        Ok(Self::new(config, transport))
    }

    /// Request a new session on top of an established connection.
    ///
    /// A connection is able to multiplex several sessions simultaneously so this method may be
    /// called more than once on a given connection. This method may fail if either the client
    /// (due to config limitiation) or the server hits a limit on the number of concurrent
    /// channels per connection.
    pub async fn session(
        &mut self,
    ) -> Result<Result<Session<Client>, ChannelOpenFailureReason>, ConnectionError> {
        let req: OpenRequest<Session<Client>> = OpenRequest { specific: () };
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

impl Drop for Connection {
    fn drop(&mut self) {
        let x = std::mem::replace(&mut self.close, oneshot::channel().0);
        x.send(DisconnectReason::BY_APPLICATION);
    }
}

impl Future for Connection {
    type Output = ConnectionError;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = Pin::into_inner(self);
        let r = ready!(Pin::new(&mut s.error).poll(cx));
        Poll::Ready(r.unwrap_or(ConnectionError::Terminated))
    }
}

pub enum ConnectionRequest {
    Global(GlobalRequest),
    OpenSession(OpenSessionRequest),
}

pub struct OpenSessionRequest {}

impl OpenSessionRequest {
    fn accept(self) -> Session<Server> {
        todo!()
    }
}

impl Stream for Connection {
    type Item = ConnectionRequest;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut self_ = self;
        Stream::poll_next(Pin::new(&mut self_.as_mut().requests_rx), cx)
    }
}
