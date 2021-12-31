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
pub(crate) use self::id::*;
pub(crate) use self::kex::*;
pub(crate) use self::msg::*;

pub use self::config::TransportConfig;
pub use self::default::DefaultTransport;
pub use self::error::TransportError;

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
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if no message is available for now.
    fn rx_peek(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>>;

    /// Consume the current rx buffer (only after `rx_peek`).
    ///
    /// The message shall have been decoded and processed before being cosumed.
    fn rx_consume(&mut self) -> Result<(), TransportError>;

    /// Try to allocate buffer space for the next message to send.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if the output buffer is too full and must be flushed first.
    fn tx_alloc(&mut self, cx: &mut Context, len: usize)
        -> Poll<Result<&mut [u8], TransportError>>;

    /// Commits the current tx buffer as ready for sending (only after `tx_alloc`).
    ///
    /// A message shall have been written to the tx buffer.
    fn tx_commit(&mut self) -> Result<(), TransportError>;

    /// Try to flush all pending operations and output buffers.
    fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>>;

    /// Try to send MSG_DISCONNECT and swallow all errors.
    ///
    /// Message delivery may silently fail on errors or if output buffer is full.
    fn tx_disconnect(
        &mut self,
        cx: &mut Context,
        reason: DisconnectReason,
    ) -> Poll<Result<(), TransportError>>;

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
            let buf = ready!(self.tx_alloc(cx, size))?;
            SshCodec::encode_into(msg, buf)?;
            self.tx_commit()?;
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
            if let Some(buf) = ready!(self.rx_peek(cx))? {
                let msg = SshCodec::decode(buf)?;
                self.rx_consume()?;
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
        poll_fn(|cx| self.tx_flush(cx)).await
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
        let buf = ready!(self.0.tx_alloc(cx, size))?;
        SshCodec::encode_into(msg, buf)?;
        self.0.tx_commit()?;
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
    fn rx_peek(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>> {
        self.0.rx_peek(cx)
    }

    #[inline]
    fn rx_consume(&mut self) -> Result<(), TransportError> {
        self.0.rx_consume()
    }

    #[inline]
    fn tx_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        self.0.tx_alloc(cx, len)
    }

    #[inline]
    fn tx_commit(&mut self) -> Result<(), TransportError> {
        self.0.tx_commit()
    }

    #[inline]
    fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.0.tx_flush(cx)
    }

    #[inline]
    fn tx_disconnect(
        &mut self,
        cx: &mut Context,
        reason: DisconnectReason,
    ) -> Poll<Result<(), TransportError>> {
        self.0.tx_disconnect(cx, reason)
    }

    #[inline]
    fn session_id(&self) -> Result<&Secret, TransportError> {
        self.0.session_id()
    }
}
