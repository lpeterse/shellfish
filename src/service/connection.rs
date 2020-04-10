mod channel;
mod channels;
mod config;
mod error;
pub mod future;
mod global;
mod msg;

pub use self::config::*;
pub use self::error::*;
pub use self::future::*;
pub use self::global::*;

use self::channel::*;
use self::channels::*;
use self::future::ConnectionFuture;
use self::msg::*;
use super::*;

use crate::client::Client;
use crate::codec::*;
use crate::role::*;
use crate::transport::{DisconnectReason, TransportLayer, TransportLayerExt};
use crate::util::manyshot;
use crate::util::oneshot;

use async_std::future::Future;
use async_std::stream::Stream;
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
    close_tx: oneshot::Sender<DisconnectReason>,
    error_rx: oneshot::Receiver<ConnectionError>,
    request_tx: manyshot::Sender<OutboundRequest>,
    request_rx: manyshot::Receiver<InboundRequest>,
}

impl Connection {
    /// Create a new connection.
    ///
    /// The connection spawns a separate handler thread. This handler thread's lifetime is linked
    /// the `Connection` object: `Drop`ping the connection will send it a termination signal.
    fn new<C: ConnectionConfig, T: TransportLayer>(config: &C, transport: T) -> Connection {
        let (close_tx, close_rx) = oneshot::channel();
        let (error_tx, error_rx) = oneshot::channel();
        let (request_in_tx, request_in_rx) = manyshot::new();
        let (request_out_tx, request_out_rx) = manyshot::new();
        let future =
            ConnectionFuture::new(config, transport, close_rx, request_out_tx, request_in_rx);
        async_std::task::spawn(async { error_tx.send(future.await) });
        Connection {
            close_tx,
            error_rx,
            request_tx: request_in_tx,
            request_rx: request_out_rx,
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

    pub async fn request2(&mut self, name: String, data: Vec<u8>) -> Result<(), ConnectionError> {
        let request = GlobalRequest::new(name, data);
        self.request_tx
            .send(OutboundRequest::Global(request))
            .await
            .ok_or(ConnectionError::Terminated)
    }

    pub async fn request_want_reply(
        &self,
        name: String,
        data: Vec<u8>,
    ) -> Result<GlobalReply, ConnectionError> {
        let (request, reply) = GlobalRequest::new_want_reply(name, data);
        self.request_tx
            .send(OutboundRequest::Global(request))
            .await
            .ok_or(ConnectionError::Terminated)?;
        Ok(reply)
    }

    /// Request a new session on top of an established connection.
    ///
    /// A connection is able to multiplex several sessions simultaneously so this method may be
    /// called more than once on a given connection. This method may fail if either the client
    /// (due to config limitiation) or the server hits a limit on the number of concurrent
    /// channels per connection.
    pub async fn open_session(
        &mut self,
    ) -> Result<Result<Session<Client>, ChannelOpenFailureReason>, ConnectionError> {
        /*
        let req: OpenRequest<Session<Client>> = OpenRequest { specific: () };
        self.requests.request(req).await
        */
        todo!()
    }

    /// Request a direct-tcpip forwarding on top of an establied connection.
    pub async fn open_direct_tcpip(
        &mut self,
        params: DirectTcpIpOpen,
    ) -> Result<Result<DirectTcpIp, ChannelOpenFailureReason>, ConnectionError> {
        let (tx, rx) = oneshot::channel();
        let req: OpenRequest<DirectTcpIp> = OpenRequest {
            open: params,
            reply: tx,
        };
        self.request_tx
            .send(OutboundRequest::OpenDirectTcpIp(req))
            .await
            .ok_or(ConnectionError::Terminated)?;
        rx.await.ok_or(ConnectionError::Terminated)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let x = std::mem::replace(&mut self.close_tx, oneshot::channel().0);
        x.send(DisconnectReason::BY_APPLICATION);
    }
}

impl Future for Connection {
    type Output = ConnectionError;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = Pin::into_inner(self);
        let r = ready!(Pin::new(&mut s.error_rx).poll(cx));
        Poll::Ready(r.unwrap_or(ConnectionError::Terminated))
    }
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

#[derive(Debug)]
pub enum InboundRequest {
    Global(GlobalRequest),
}

#[derive(Debug)]
pub(crate) enum OutboundRequest {
    Global(GlobalRequest),
    OpenSession(OpenRequest<Session<Client>>),
    OpenDirectTcpIp(OpenRequest<DirectTcpIp>),
}

#[derive(Debug)]
pub(crate) struct OpenRequest<T: ChannelOpen> {
    open: <T as ChannelOpen>::Open,
    reply: oneshot::Sender<Result<T, ChannelOpenFailureReason>>,
}

impl Stream for Connection {
    type Item = InboundRequest;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.request_rx.poll_receive(cx)
    }
}
