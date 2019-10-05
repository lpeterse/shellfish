mod buffered_receiver;
mod buffered_sender;
mod encryption;
mod error;
mod identification;
mod kex;
mod key_streams;
mod msg_debug;
mod msg_disconnect;
mod msg_ignore;
mod msg_service_accept;
mod msg_service_request;
mod msg_unimplemented;
mod packet_layout;
mod session_id;
mod transmitter;

pub use self::buffered_receiver::*;
pub use self::buffered_sender::*;
pub use self::encryption::*;
pub use self::error::*;
pub use self::identification::*;
pub use self::kex::*;
pub use self::key_streams::*;
pub use self::msg_debug::*;
pub use self::msg_disconnect::*;
pub use self::msg_ignore::*;
pub use self::msg_service_accept::*;
pub use self::msg_service_request::*;
pub use self::msg_unimplemented::*;
pub use self::packet_layout::*;
pub use self::session_id::*;
pub use self::transmitter::*;

use crate::client::Client;
use crate::codec::*;
use crate::role::*;
use crate::socket::*;

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

pub struct TransportConfig {
    identification: Identification,
    kex_interval_bytes: u64,
    kex_interval_duration: std::time::Duration,
    alive_interval: std::time::Duration,
    inactivity_timeout: std::time::Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            identification: Identification::default(),
            kex_interval_bytes: 1_000_000_000,
            kex_interval_duration: std::time::Duration::from_secs(3600),
            alive_interval: std::time::Duration::from_secs(3),
            inactivity_timeout: std::time::Duration::from_secs(10),
        }
    }
}

pub struct Transport<R: Role, T> {
    transmitter: Transmitter<T>,

    kex: <R as HasTransport>::KexMachine,
}

impl<R: Role, T: Socket> Transport<R, T> {
    /// Create a new transport.
    ///
    /// The initial key exchange has been completed successfully when this
    /// function does not return an error.
    pub async fn new(config: &TransportConfig, stream: T) -> Result<Self, TransportError> {
        let mut transport = Self {
            transmitter: Transmitter::new(config, stream).await?,
            kex: <R as HasTransport>::KexMachine::new(
                config.kex_interval_bytes,
                config.kex_interval_duration,
            ),
        };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Initiate a rekeying and wait for it to complete.
    pub async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex.init_local();
        poll_fn(|cx| self.poll_internal(cx)).await
    }

    /// Return the session id belonging to the connection.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    pub fn session_id(&self) -> &SessionId {
        &self.kex.session_id()
    }

    /// Send a message.
    pub async fn send<M: Encode>(&mut self, msg: &M) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_send(cx, msg)).await
    }

    /// Receive a message.
    /// 
    /// Actual decoding and consumption shall be done with `decode()` and `consume()`.
    /// This function returns `Ok(())` until all messages pending have been consumed.
    /// 
    /// NB: It is necessary to poll the send or receive methods to keep the transport processing.
    pub async fn receive(&mut self) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_receive(cx)).await
    }

    /// Flush the transport.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        self.transmitter.flush().await
    }

    /// Check whether the transport is flushed.
    pub fn is_flushed(&self) -> bool {
        self.transmitter.flushed()
    }

    pub async fn request_service(mut self, service_name: &str) -> Result<Self, TransportError> {
        let req = MsgServiceRequest(service_name);
        self.send(&req).await?;
        self.flush().await?;
        self.receive().await?;
        let _: MsgServiceAccept<'_> = self.decode_ref().ok_or(TransportError::MessageUnexpected)?;
        self.consume();
        Ok(self)
    }

    pub fn decode<Msg: Decode>(&mut self) -> Option<Msg> {
        self.transmitter.decode()
    }

    pub fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        self.transmitter.decode()
    }

    pub fn consume(&mut self) {
        self.transmitter.consume()
    }

    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.transmitter.poll_flush(cx)
    }

    pub fn poll_send<Msg: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        ready!(self.poll_internal(cx))?;
        self.transmitter.poll_send(cx, msg)

    }

    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.poll_internal(cx))?;
        self.transmitter.poll_receive(cx)
    }

    // Poll all internal processes like kex and timers. Returns `Poll::Ready` when all processes
    // are completed (kex is complete and timers have not fired).
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
