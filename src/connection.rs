mod config;
mod error;
mod handler;
mod msg;
mod request;
mod state;

pub mod channel;
pub mod global;

pub use self::config::ConnectionConfig;
pub use self::error::ConnectionError;
pub use self::handler::ConnectionHandler;

use self::channel::DirectTcpIp;
use self::channel::DirectTcpIpParams;
use self::channel::OpenFailure;
use self::channel::Session;
use self::error::ConnectionErrorWatch;
use self::global::{Global, GlobalWantReply};
use self::request::ConnectionRequest;
use self::state::ConnectionState;
use crate::transport::{GenericTransport, TransportError};
use crate::util::codec::*;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;

/// The `ssh-connection` service offers channel multiplexing for a variety of applications like
/// remote shell and command execution as well as TCP/IP port forwarding and various other
/// extensions.
///
/// Unless client or server request a service on top of this protocol the connection just keeps
/// itself alive and does nothing.
///
/// [Connection] implements [Clone]: All clones are equal and cloning is quite cheap. Dropping the
/// last clone will close the connection and free all resources. It will also
/// terminate all dependant channels (shells and forwardings etc). Be aware that the connection
/// is not closed on drop unless there are no more clones; [Connection::close] it explicitly
/// if you need to.
#[derive(Clone, Debug)]
pub struct Connection {
    creqs: mpsc::Sender<ConnectionRequest>,
    close: Arc<Mutex<oneshot::Receiver<()>>>,
    error: ConnectionErrorWatch,
}

impl Connection {
    /// Create a new connection (you may want to use
    /// [Client::connect()](crate::client::Client::connect) instead).
    ///
    /// The connection spawns a separate handler task. The task and the inner connection state only
    /// lives as long as the connection is alive. All operations on dead connection are supposed to
    /// return the error which caused the connection to die. The error is preserved as long as there
    /// are references to the connection.
    pub fn new<F: FnOnce(&Self) -> Box<dyn ConnectionHandler>>(
        config: &Arc<ConnectionConfig>,
        transport: GenericTransport,
        handle: F,
    ) -> Self {
        let (r1, r2) = mpsc::channel(1);
        let (c1, c2) = oneshot::channel();
        let (e1, e2) = watch::channel(None);
        let self_ = Self {
            creqs: r1,
            close: Arc::new(Mutex::new(c2)),
            error: e2.clone(),
        };
        let hb = handle(&self_);
        let cs = ConnectionState::new(config, hb, transport, r2, c1, e1, e2);
        drop(spawn(cs));
        self_
    }

    /// Open a new `session` channel.
    pub async fn open_session(&self) -> Result<Result<Session, OpenFailure>, ConnectionError> {
        let (s, r) = oneshot::channel();
        let req = ConnectionRequest::OpenSession { reply: s };
        self.creqs
            .send(req)
            .await
            .map_err(|_| self.error_or_dropped())?;
        r.await.map_err(|_| self.error_or_dropped())
    }

    /// Open a new `direct-tcpip` channel.
    pub async fn open_direct_tcpip(
        &self,
        params: DirectTcpIpParams,
    ) -> Result<Result<DirectTcpIp, OpenFailure>, ConnectionError> {
        let (s, r) = oneshot::channel();
        let req = ConnectionRequest::OpenDirectTcpIp {
            data: SshCodec::encode(&params).ok_or(TransportError::InvalidEncoding)?,
            reply: s,
        };
        self.creqs
            .send(req)
            .await
            .map_err(|_| self.error_or_dropped())?;
        r.await.map_err(|_| self.error_or_dropped())
    }

    /// Perform a global request (without reply).
    pub async fn request<T: Global>(&self, data: &T::RequestData) -> Result<(), ConnectionError> {
        let request = ConnectionRequest::Global {
            name: T::NAME,
            data: SshCodec::encode(data).ok_or(TransportError::InvalidEncoding)?,
            reply: None,
        };
        self.creqs
            .send(request)
            .await
            .map_err(|_| self.error_or_dropped())
    }

    /// Perform a global request and wait for the reply.
    pub async fn request_want_reply<T: GlobalWantReply>(
        &self,
        data: &T::RequestData,
    ) -> Result<Result<T::ResponseData, ()>, ConnectionError> {
        let (reply, response) = oneshot::channel();
        let request = ConnectionRequest::Global {
            name: T::NAME,
            data: SshCodec::encode(data).ok_or(TransportError::InvalidEncoding)?,
            reply: Some(reply),
        };
        self.creqs
            .send(request)
            .await
            .map_err(|_| self.error_or_dropped())?;
        match response.await.map_err(|_| self.error_or_dropped())? {
            Err(()) => Ok(Err(())),
            Ok(vec) => Ok(Ok(
                SshCodec::decode(&vec).ok_or(TransportError::InvalidEncoding)?
            )),
        }
    }

    /// Check whether the connection terminated with an error.
    ///
    /// This does not actively check whether the peer is still reachable nor does it guarantee
    /// that subsequent operations on the connection would succeed.
    pub fn check(&self) -> Result<(), ConnectionError> {
        if let Some(e) = self.error.borrow().as_ref() {
            Err(e.as_ref().clone())
        } else {
            Ok(())
        }
    }

    /// Check the connection by sending a global keep-alive request and awaiting its reply.
    ///
    /// Returns the error that terminated the connection in case the roundtrip does not succeed.
    pub async fn check_with_keepalive(&self) -> Result<(), ConnectionError> {
        // Ignore whether the peer actually accepts or rejects the request.
        // Both alternatives imply a healthy connection.
        let _ = self.request_want_reply::<global::KeepAlive>(&()).await?;
        Ok(())
    }

    /// Close the connection.
    ///
    /// This function is intentionally non-async and may be used in implemenations of [Drop].
    ///
    /// This tells the handler task to try to send a disconnect message to the peer
    /// (best effort/won't block for security reasons) and then terminate itself. The disconnect
    /// has highest priority - the handler task will not do anything else that might block the
    /// disconnection process.
    ///
    /// Hint: Use [closed](Self::closed) in order to await actual disconnection.
    pub fn close(&mut self) {
        self.close.lock().unwrap().close();
    }

    /// Wait for the connection being closed (does not actively close it!).
    pub async fn closed(&mut self) {
        loop {
            if self.error.borrow().is_some() {
                return;
            }
            if self.error.changed().await.is_err() {
                return;
            }
        }
    }

    fn error(&self) -> Option<ConnectionError> {
        self.error.borrow().as_deref().map(|x| x.clone())
    }

    fn error_or_dropped(&self) -> ConnectionError {
        self.error().unwrap_or(ConnectionError::Dropped)
    }
}
