mod channel;
mod config;
mod error;
mod future;
mod global;
mod msg;
mod request;
mod state;

pub use self::channel::*;
pub use self::config::*;
pub use self::error::*;
pub use self::global::*;
pub use self::msg::ChannelOpenFailure;
pub use self::request::ConnectionRequest;

use self::future::*;
use self::msg::*;
pub use self::state::*;

use crate::client::Client;
use crate::transport::{Service, DisconnectReason, Transport, TransportLayer};
use crate::util::codec::*;
use crate::util::oneshot;

use async_std::future::Future;
use async_std::stream::Stream;
use async_std::task::{Context, Poll, Waker};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// The `ssh-connection` service offers channel multiplexing for a variety of applications like remote
/// shell and command execution as well as TCP/IP port forwarding and various other extensions.
///
/// Unless client or server request a service on top of this protocol the connection just keeps
/// itself alive and does nothing. Dropping the connection object will close the connection and
/// free all resources. It will also terminate all dependant channels (shells and forwardings etc).
#[derive(Debug)]
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

    /// Perform a global request (without reply).
    pub fn request<N: Into<String>, D: Into<Vec<u8>>>(&mut self, name: N, data: D) {
        self.with_state(|x| x.request(name.into(), data.into()))
    }

    /// Perform a global request and return future resolving on peer response.
    pub fn request_want_reply<N: Into<String>, D: Into<Vec<u8>>>(
        &mut self,
        name: N,
        data: D,
    ) -> GlobalReplyFuture {
        self.with_state(|x| x.request_want_reply(name.into(), data.into()))
    }

    /// Request a new channel on top of an established connection.
    pub fn open<C: Channel>(&self, params: C::Open) -> ChannelOpenFuture<C> {
        self.with_state(|x| x.open(params))
    }

    /// Request a new session on top of an established connection.
    pub fn open_session(&self) -> ChannelOpenFuture<Session<Client>> {
        self.open(())
    }

    /// Request a direct-tcpip forwarding on top of an established connection.
    pub fn open_direct_tcpip<S: Into<String>>(
        &self,
        dst_host: S,
        dst_port: u16,
        src_addr: std::net::IpAddr,
        src_port: u16,
    ) -> ChannelOpenFuture<DirectTcpIp> {
        self.open(DirectTcpIpOpen {
            dst_host: dst_host.into(),
            dst_port,
            src_addr,
            src_port,
        })
    }

    /// Perform the given operation on the Mutex-protected connection state.
    /// Wakeup the connection future task afterwards (if necessary).
    ///
    /// NB: This seemingly complicated mechanism's intention is to wakeup the
    /// other task _after_ the Mutex lock has been released. Mutexes/Futexes are
    /// cheap unless they are contended. This tries to minimize the contention
    /// by not waking up the other task as long as we still hold the lock.
    fn with_state<F, X>(&self, f: F) -> X
    where
        F: FnOnce(&mut ConnectionState<T>) -> X,
    {
        let (result, waker) = {
            let mut state = self.0.lock().unwrap();
            (f(&mut state), state.inner_task_waker())
        };
        let _ = waker.map(Waker::wake);
        result
    }
}

impl<T: TransportLayer> Clone for Connection<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: TransportLayer> Drop for Connection<T> {
    fn drop(&mut self) {
        self.with_state(|x| x.flag_inner_task_for_wakeup())
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
    type Item = ConnectionRequest;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.with_state(|x| {
            if let Some(request) = x.next() {
                return Poll::Ready(Some(request));
            }
            if x.result().is_some() {
                return Poll::Ready(None);
            }
            x.register_outer_task(cx);
            Poll::Pending
        })
    }
}

impl<T: TransportLayer> Future for Connection<T> {
    type Output = Result<DisconnectReason, ConnectionError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.with_state(|x| {
            if let Some(result) = x.result() {
                return Poll::Ready(result);
            }
            while let Some(request) = x.next() {
                drop(request)
            }
            x.register_outer_task(cx);
            Poll::Pending
        })
    }
}
