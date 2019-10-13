pub(crate) mod buffered_receiver;
pub(crate) mod buffered_sender;
pub(crate) mod config;
pub(crate) mod cipher;
pub(crate) mod error;
pub(crate) mod identification;
pub(crate) mod kex;
pub(crate) mod key_streams;
pub(crate) mod msg_debug;
pub(crate) mod msg_disconnect;
pub(crate) mod msg_ignore;
pub(crate) mod msg_service_accept;
pub(crate) mod msg_service_request;
pub(crate) mod msg_unimplemented;
pub(crate) mod packet_layout;
pub(crate) mod session_id;
pub(crate) mod socket;
pub(crate) mod transmitter;

pub use self::config::*;
pub use self::error::*;
pub use self::identification::*;
pub use self::session_id::*;
pub use self::socket::*;

use self::buffered_receiver::*;
use self::buffered_sender::*;
use self::cipher::*;
use self::kex::*;
use self::key_streams::*;
use self::msg_debug::*;
use self::msg_disconnect::*;
use self::msg_ignore::*;
use self::msg_service_accept::*;
use self::msg_service_request::*;
use self::msg_unimplemented::*;
use self::packet_layout::*;

use crate::client::Client;
use crate::codec::*;
use crate::role::*;

use futures::future::poll_fn;
use futures::future::FutureExt;
use futures::io::{ReadHalf, WriteHalf};
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures_timer::Delay;
use std::convert::From;
use std::marker::Unpin;
use std::option::Option;
use std::pin::Pin;

pub trait HasTransport {
    type KexMachine: KexMachine + Sized + Send + Unpin;
}

impl HasTransport for Client {
    type KexMachine = ClientKexMachine;
}

pub struct Transport<R: Role, S> {
    transmitter: Transmitter<S>,
    kex: <R as HasTransport>::KexMachine,
}

impl<R: Role, S: Socket> Transport<R, S> {
    /// Create a new transport.
    ///
    /// The initial key exchange has been completed successfully when this
    /// function does not return an error.
    pub async fn new<C: TransportConfig>(config: &C, socket: S) -> Result<Self, TransportError> {
        let mut transport = Self {
            transmitter: Transmitter::new(config, socket).await?,
            kex: <R as HasTransport>::KexMachine::new(config),
        };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Return the connection's session id.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    pub fn session_id(&self) -> &Option<SessionId> {
        &self.kex.session_id()
    }

    /// Check whether the transport is flushed (output buffer empty).
    pub fn is_flushed(&self) -> bool {
        self.transmitter.flushed()
    }

    /// Initiate a rekeying and wait for it to complete.
    pub async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex.init_local();
        poll_fn(|cx| self.poll_internal(cx)).await
    }

    /// Send a message.
    pub async fn send<M: Encode>(&mut self, msg: &M) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_send(cx, msg)).await
    }

    /// Receive a message.
    pub async fn receive(&mut self) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_receive(cx)).await
    }

    /// Flush the transport.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_flush(cx)).await
    }

    /// Request a service by name.
    ///
    /// Service requests either succeeed or the connection is terminated by a disconnect message.
    pub async fn request_service(mut self, service_name: &str) -> Result<Self, TransportError> {
        let msg = MsgServiceRequest(service_name);
        self.send(&msg).await?;
        self.flush().await?;
        self.receive().await?;
        let _: MsgServiceAccept<'_> = self.decode_ref().ok_or(TransportError::MessageUnexpected)?;
        self.consume();
        Ok(self)
    }

    /// Try to decode the current message (only after `receive` or `poll_receive`).
    pub fn decode<Msg: Decode>(&mut self) -> Option<Msg> {
        self.transmitter.decode()
    }

    /// Try to decode the current message (only after `receive` or `poll_receive`).
    ///
    /// In contrast to `decode` this method is able to decode messages that hold references into
    /// the receive buffer and may be used to avoid temporary heap allocations. Unfortunately,
    /// this borrows the transport reference which cannot be used until the message gets dropped.
    pub fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        self.transmitter.decode()
    }

    /// Consumes the current message (only after `receive` or `poll_receive`).
    ///
    /// The message shall have been decoded and processed before being cosumed.
    pub fn consume(&mut self) {
        self.transmitter.consume()
    }

    /// Try to flush all pending operations and output buffers.
    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.poll_internal(cx))?;
        self.transmitter.poll_flush(cx)
    }

    /// Try to send a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if the output buffer is too full and must be flushed first.
    ///
    /// NB: Polling drives the internal processes to completion.
    pub fn poll_send<Msg: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        ready!(self.poll_internal(cx))?;
        self.transmitter.poll_send(cx, msg)
    }

    /// Try to receive a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if now message is available for now.
    ///
    /// NB: Polling drives the internal processes to completion.
    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.poll_internal(cx))?;
        self.transmitter.poll_receive(cx)
    }

    pub fn poll_send_unimplemented(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        todo!()
    }

    /// Poll all internal processes like kex and timers. Returns `Poll::Ready` when all processes
    /// are completed (kex is complete and timers have not yet fired).
    fn poll_internal(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        loop {
            // The inactivity check causes an exception in case of timeout and falls through else.
            // Calling it also registers the timer for wakeup (consider this when reordering code).
            self.transmitter.check_inactivity_timeout(cx)?;
            // Send a keep alive message when determined to be required.
            // Like above, the call registers the timer for wakeup. The alive timer is reset
            // automatically when the message has been sent successfully.
            if self.transmitter.check_keep_alive_required(cx)? {
                ready!(self.transmitter.poll_send(cx, &MsgIgnore::new()))?;
                log::debug!("Sent MSG_IGNORE (as keep-alive)");
                ready!(self.transmitter.poll_flush(cx))?;
            }
            if self.kex.is_in_progress(cx, &mut self.transmitter)? {
                ready!(self.kex.poll_flush(cx, &mut self.transmitter))?;
                ready!(self.transmitter.poll_receive(cx))?;
                // Try to interpret as MSG_DISCONNECT. If successful, convert it into an error and let
                // the callee handle the termination.
                match self.decode_ref() {
                    Some(x) => {
                        let _: MsgDisconnect = x;
                        log::debug!("Received MSG_DISCONNECT");
                        return Poll::Ready(Err(TransportError::DisconnectByPeer(x.reason)));
                    }
                    None => (),
                }
                // Try to interpret as MSG_IGNORE. If successful, the message is (as the name suggests)
                // just ignored. Ignore messages may be introduced any time to impede traffic analysis
                // and for keep alive.
                match self.decode_ref() {
                    Some(x) => {
                        let _: MsgIgnore = x;
                        log::debug!("Received MSG_IGNORE");
                        self.consume();
                        continue;
                    }
                    None => (),
                }
                // Try to interpret as MSG_UNIMPLEMENTED. If successful, convert this into an error.
                match self.decode() {
                    Some(x) => {
                        let _: MsgUnimplemented = x;
                        log::debug!("Received MSG_UNIMPLEMENTED");
                        return Poll::Ready(Err(TransportError::MessageUnimplemented));
                    }
                    None => (),
                }
                // Try to interpret as MSG_DEBUG. If successful, log as debug and continue.
                match self.decode_ref() {
                    Some(x) => {
                        let _: MsgDebug = x;
                        log::debug!("Received MSG_DEBUG: {:?}", x.message);
                        self.consume();
                        continue;
                    }
                    None => (),
                }
                // Try to interpret as MSG_KEX_INIT. If successful, pass it to the kex handler.
                // Unless the protocol is violated, kex is in progress afterwards (if not already).
                match self.decode() {
                    Some(msg) => {
                        log::debug!("Received MSG_KEX_INIT");
                        self.kex.init_remote(msg)?;
                        self.consume();
                        continue;
                    }
                    None => (),
                }
                // After remote sent a MSG_KEX_INIT packet no other packets than those handled above
                // and kex-related packets are allowed. We therefor route all packets to the kex
                // handler. The kex handler is supposed to cause an exception on unrecognized packets.
                if self.kex.is_init_received() {
                    self.kex.consume(&mut self.transmitter)?;
                    continue;
                }
                // Kex is in progress, but the KEX_INIT packet from remote has not been received yet
                // which means that other packets may arrive before.
            }
            return Poll::Ready(Ok(()));
        }
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
