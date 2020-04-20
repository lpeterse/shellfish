mod channel;
mod config;
mod error;
mod future;
mod global;
mod msg;
mod state;

pub use self::channel::*;
pub use self::config::*;
pub use self::error::*;
pub use self::global::*;
pub use self::msg::ChannelOpenFailureReason;
pub use self::state::ConnectionRequest;

use self::future::*;
use self::msg::*;
use self::state::*;

use crate::client::Client;
use crate::codec::*;
use crate::service::Service;
use crate::transport::{DisconnectReason, Transport, TransportLayer};
use crate::util::oneshot;

use async_std::stream::Stream;
use async_std::task::ready;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// The connection protocol offers channel multiplexing for a variety of applications like remote
/// shell and command execution as well as TCP/IP port forwarding and various other extensions.
///
/// Unless client or server request a service on top of this protocol the connection just keeps
/// itself alive and does nothing. Dropping the connection object will close the connection and
/// free all resources. It will also terminate all dependant channels (shells and forwardings etc).
pub struct Connection<T: TransportLayer = Transport>(Arc<Mutex<ConnectionState<T>>>);

impl<T: TransportLayer> Connection<T> {
    /// Create a new connection.
    ///
    /// The connection spawns a separate handler thread. This handler thread's lifetime is linked
    /// the `Connection` object: `Drop`ping the connection will send it a termination signal.
    fn new(config: &Arc<ConnectionConfig>, transport: T) -> Self {
        let state = ConnectionState::new(config, transport);
        let state = Arc::new(Mutex::new(state));
        let future = ConnectionFuture::new(&state);
        async_std::task::spawn(future);
        Self(state)
    }

    pub fn request<N: Into<String>, D: Into<Vec<u8>>>(&mut self, name: N, data: D) {
        let mut x = self.0.lock().unwrap();
        x.request(name.into(), data.into())
    }

    pub fn request_want_reply<N: Into<String>, D: Into<Vec<u8>>>(
        &mut self,
        name: N,
        data: D,
    ) -> GlobalReplyFuture {
        let mut x = self.0.lock().unwrap();
        x.request_want_reply(name.into(), data.into())
    }

    /// Request a new session on top of an established connection.
    ///
    /// A connection is able to multiplex several sessions simultaneously so this method may be
    /// called more than once on a given connection. This method may fail if either the client
    /// (due to config limitiation) or the server hits a limit on the number of concurrent
    /// channels per connection.
    pub fn open_session(&mut self) -> ChannelOpenFuture<Session<Client>> {
        let mut x = self.0.lock().unwrap();
        x.open(())
    }

    /// Request a direct-tcpip forwarding on top of an establied connection.
    pub fn open_direct_tcpip<S: Into<String>>(
        &mut self,
        dst_host: S,
        dst_port: u16,
        src_addr: std::net::IpAddr,
        src_port: u16,
    ) -> ChannelOpenFuture<DirectTcpIp> {
        let mut x = self.0.lock().unwrap();
        x.open(DirectTcpIpOpen {
            dst_host: dst_host.into(),
            dst_port: dst_port as u32,
            src_addr: src_addr.to_string(),
            src_port: src_port as u32,
        })
    }
}

impl<T: TransportLayer> Drop for Connection<T> {
    fn drop(&mut self) {
        let mut x = self.0.lock().unwrap();
        x.disconnect(DisconnectReason::BY_APPLICATION)
    }
}

impl<T: TransportLayer> Service for Connection<T> {
    type Config = ConnectionConfig;
    type Transport = T;

    const NAME: &'static str = "ssh-connection";

    fn new(config: &Arc<Self::Config>, transport: T) -> Self {
        Self::new(config, transport)
    }
}

impl<T: TransportLayer> Stream for Connection<T> {
    type Item = Result<ConnectionRequest, ConnectionError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut x = self.0.lock().unwrap();
        Poll::Ready(Some(ready!(x.poll_next(cx))))
    }
}
