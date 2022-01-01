pub(crate) mod config;
pub(crate) mod cookie;
pub(crate) mod crypto;
pub(crate) mod default;
pub(crate) mod disconnect;
pub(crate) mod ecdh;
pub(crate) mod error;
pub(crate) mod id;
pub(crate) mod kex;
pub(crate) mod keys;
pub(crate) mod msg;
pub(crate) mod transceiver;

pub(crate) use self::crypto::*;
pub(crate) use self::disconnect::*;
pub(crate) use self::ecdh::*;
pub(crate) use self::kex::*;
pub(crate) use self::msg::*;

pub use self::config::TransportConfig;
pub use self::default::DefaultTransport;
pub use self::error::TransportError;
pub use self::id::Identification;

use crate::agent::*;
use crate::host::*;
use crate::ready;
use crate::util::codec::*;
use crate::util::poll_fn;
use crate::util::secret::Secret;

use std::convert::From;
use std::fmt::Debug;
use std::option::Option;
use std::sync::Arc;
use std::task::{Context, Poll};

pub trait Transport: Debug + Send + Unpin + 'static {
    /// Try to receive the next message.
    ///
    /// Any message received MUST be dispatched _and_ [consumed](Self::consume).
    /// It is critical to call this function in order to guarantee progress and drive internal
    /// tasks like kex.
    ///
    /// - Returns `Pending` during key exchange (will unblock as soon as kex is complete).
    /// - Returns `Ready(Ok(Some(msg)))` for each inbound non-transport layer message.
    /// - Returns `Ready(Ok(None))` if no message is available for now.
    /// - Returns `Ready(Err(_))` for all errors (internal state is undefined afterwards and
    ///   instance must not be used).
    fn poll_next(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>>;

    /// Consume the current rx buffer (only after [Self::rx_peek]).
    ///
    /// The message shall have been decoded and processed before being cosumed.
    fn consume_next(&mut self) -> Result<(), TransportError>;

    /// Try to allocate buffer space for the next message to send.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if the output buffer is too full and must be flushed first.
    fn poll_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>>;

    /// Commits the current tx buffer as ready for sending (only after `tx_alloc`).
    ///
    /// A message shall have been written to the tx buffer.
    fn commit_alloc(&mut self) -> Result<(), TransportError>;

    /// Try to flush all pending operations and output buffers.
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>>;


    /// Return the connection's session id.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    fn session_id(&self) -> Result<&Secret, TransportError>;
}

/// A box wrapper around all types that implement [Transport] with extra async methods.
///
/// This is the type you should be working with in generic code like service extensions etc.
/// Decoupling the real transport by a level of indirection is also useful for testing.
///
/// Serveral convenient async methods for sending and receiving are supplied. They are easy to work
/// with, but do not offer features like direct buffer access which is required for highest
/// performance. In such a case the re-exposed low-level [Transport] methods need to be used.
#[derive(Debug)]
pub struct GenericTransport(Box<dyn Transport>);

impl GenericTransport {
    /// Create a new boxed transport object.
    ///
    /// The passed transport should already be connected as this type does not offer methods
    /// for the establishment of connections. Connected transports are role-agnostic: Both the
    /// client and server sides behave exactly the same form a users perspective (just like
    /// network sockets).
    pub fn from<T: Transport>(transport: T) -> Self {
        Self(Box::new(transport))
    }

    /// Send a message.
    ///
    /// You may only send non-transport message or the behavior is undefined.
    /// Sending a message only enqueues it for transmission, but it is not guaranteed that it has
    /// been transmitted when this function returns. Use [flush](Self::flush) when you want all
    /// queued messages to be transmitted (but use it wisely).
    pub async fn send<M: SshEncode>(&mut self, msg: &M) -> Result<(), TransportError> {
        poll_fn(|cx| {
            let size = SshCodec::size(msg)?;
            let buf = ready!(self.poll_alloc(cx, size))?;
            SshCodec::encode_into(msg, buf)?;
            self.commit_alloc()?;
            Poll::Ready(Ok(()))
        })
        .await
    }

    /// Receive a message.
    ///
    /// You will only receive non-transport messages (others are dispatched internally).
    /// The receive call blocks until either the next non-transport message arrives or an error
    /// occurs.
    pub async fn receive<M: SshDecode>(&mut self) -> Result<M, TransportError> {
        poll_fn(|cx| {
            if let Some(buf) = ready!(self.poll_next(cx))? {
                let msg = SshCodec::decode(buf)?;
                self.consume_next()?;
                Poll::Ready(Ok(msg))
            } else {
                Poll::Pending
            }
        })
        .await
    }

    /// Flush all output buffers.
    ///
    /// When the function returns without an error it is guaranteed that all messages that have
    /// previously been enqueued with [send](Self::send) have been transmitted and all output
    /// buffers are empty. Of course, this does not imply anything about successful reception of
    /// those messages.
    ///
    /// Do not flush too deliberately! The transport will automatically transmit the output buffers
    /// when you continue filling them by sending messages. Flush shall rather be used when
    /// you sent something like a request and need to make sure it has been transmitted before
    /// starting to wait for the response.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_flush(cx)).await
    }

    /// Request a service by name.
    ///
    /// Service requests either succeed or the connection gets terminated with a disconnect message.
    /// You cannot re-try with another service (which is why the method consumes `self`).
    ///
    /// Although any service might be requested by this method, in reality it is only really
    /// useful for requesting the `ssh-userauth` service which in turn requests another service.
    /// Requesting a service through this method means you're requesting a public/anonymous service.
    pub async fn request_service(mut self, service_name: &str) -> Result<Self, TransportError> {
        let msg = MsgServiceRequest(service_name);
        self.send(&msg).await?;
        log::debug!("Tx MSG_SERVICE_REQUEST");
        self.flush().await?;
        self.receive::<MsgServiceAccept>().await?;
        log::debug!("Rx MSG_SERVICE_ACCEPT");
        Ok(self)
    }

    pub fn poll_send<M: SshEncode>(
        &mut self,
        cx: &mut Context,
        msg: &M,
    ) -> Poll<Result<(), TransportError>> {
        let size = SshCodec::size(msg)?;
        let buf = ready!(self.0.poll_alloc(cx, size))?;
        SshCodec::encode_into(msg, buf)?;
        self.0.commit_alloc()?;
        Poll::Ready(Ok(()))
    }

    pub fn poll_send_unimplemented(
        &mut self,
        _cx: &mut Context,
    ) -> Poll<Result<(), TransportError>> {
        panic!("FIXME")
    }
}

impl Transport for GenericTransport {
    #[inline]
    fn poll_next(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>> {
        self.0.poll_next(cx)
    }

    #[inline]
    fn consume_next(&mut self) -> Result<(), TransportError> {
        self.0.consume_next()
    }

    #[inline]
    fn poll_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        self.0.poll_alloc(cx, len)
    }

    #[inline]
    fn commit_alloc(&mut self) -> Result<(), TransportError> {
        self.0.commit_alloc()
    }

    #[inline]
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.0.poll_flush(cx)
    }

    #[inline]
    fn session_id(&self) -> Result<&Secret, TransportError> {
        self.0.session_id()
    }
}
