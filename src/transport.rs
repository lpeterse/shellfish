pub(crate) mod buffered_receiver;
pub(crate) mod buffered_sender;
pub(crate) mod cipher;
pub(crate) mod config;
pub(crate) mod cookie;
pub(crate) mod ecdh_algorithm;
pub(crate) mod ecdh_hash;
pub(crate) mod error;
pub(crate) mod host_key_verification;
pub(crate) mod identification;
pub(crate) mod kex;
pub(crate) mod key_streams;
pub(crate) mod msg_debug;
pub(crate) mod msg_disconnect;
pub(crate) mod msg_ecdh_init;
pub(crate) mod msg_ecdh_reply;
pub(crate) mod msg_ignore;
pub(crate) mod msg_kex_init;
pub(crate) mod msg_new_keys;
pub(crate) mod msg_service_accept;
pub(crate) mod msg_service_request;
pub(crate) mod msg_unimplemented;
pub(crate) mod packet;
pub(crate) mod session_id;
pub(crate) mod socket;
pub(crate) mod transmitter;

pub use self::config::*;
pub use self::error::*;
pub use self::host_key_verification::*;
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
use self::packet::*;

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
        let transmitter = Transmitter::new(config, socket).await?;
        let kex = <R as HasTransport>::KexMachine::new(config, transmitter.remote_id().clone());
        let mut transport = Self { transmitter, kex };
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
        self.kex.init();
        Ok(())
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
        log::debug!("Sent MSG_SERVICE_REQUEST");
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
        ready!(self.transmitter.poll_inactivity(cx))?;
        self.transmitter.poll_flush(cx)
    }

    /// Try to send a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if the output buffer is too full and must be flushed first.
    ///
    /// NB: Polling drives kex to completion.
    pub fn poll_send<M: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &M,
    ) -> Poll<Result<(), TransportError>> {
        ready!(self.transmitter.poll_keepalive(cx))?;
        ready!(self.transmitter.poll_inactivity(cx))?;
        loop {
            ready!(self.poll_kex(cx))?;
            if self.kex.is_sending_critical() {
                ready!(self.transmitter.poll_receive(cx))?;
                if self.consume_transport_message()? {
                    continue;
                } else {
                    return self.poll_send_unimplemented(cx);
                }
            }
            return self.transmitter.poll_send(cx, &msg);
        }
    }

    /// Try to receive a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if no message is available for now.
    ///
    /// NB: Polling drives kex to completion.
    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.transmitter.poll_keepalive(cx))?;
        ready!(self.transmitter.poll_inactivity(cx))?;
        loop {
            ready!(self.poll_kex(cx))?;
            ready!(self.transmitter.poll_receive(cx))?;
            if self.consume_transport_message()? {
                continue;
            } else if self.kex.is_receiving_critical() {
                return self.poll_send_unimplemented(cx);
            } else {
                return Poll::Ready(Ok(()));
            }
        }
    }

    fn poll_kex(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let k = &mut self.kex;
        let t = &mut self.transmitter;
        k.poll(cx, t.bytes_sent(), t.bytes_received(), |cx, x| {
            match x {
                KexOutput::Init(msg) => {
                    ready!(t.poll_send(cx, &msg))?;
                    log::debug!("Sent MSG_KEX_INIT");
                }
                KexOutput::EcdhInit(msg) => {
                    ready!(t.poll_send(cx, &msg))?;
                    log::debug!("Sent MSG_ECDH_INIT");
                }
                KexOutput::EcdhReply(msg) => {
                    ready!(t.poll_send(cx, &msg))?;
                    log::debug!("Sent MSG_ECDH_REPLY");
                }
                KexOutput::NewKeys(c) => {
                    ready!(t.poll_send(cx, &MsgNewKeys {}))?;
                    log::debug!("Sent MSG_NEWKEYS");
                    t.encryption_ctx()
                        .update(
                            c.encryption_algorithm,
                            c.compression_algorithm,
                            c.mac_algorithm,
                            &mut c.key_streams.clone().c(),
                        )
                        .ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                }
            }
            t.poll_flush(cx) // TODO
        })
    }

    fn consume_transport_message(&mut self) -> Result<bool, TransportError> {
        // Try to interpret as MSG_DISCONNECT. If successful, convert it into an error and let
        // the callee handle the termination.
        match self.decode_ref() {
            Some(x) => {
                let _: MsgDisconnect = x;
                log::debug!("Received MSG_DISCONNECT");
                Err(TransportError::DisconnectByPeer(x.reason))?;
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
                return Ok(true);
            }
            None => (),
        }
        // Try to interpret as MSG_UNIMPLEMENTED. If successful, convert this into an error.
        match self.decode() {
            Some(x) => {
                let _: MsgUnimplemented = x;
                log::debug!("Received MSG_UNIMPLEMENTED");
                Err(TransportError::MessageUnimplemented)?;
            }
            None => (),
        }
        // Try to interpret as MSG_DEBUG. If successful, log as debug and continue.
        match self.decode_ref() {
            Some(x) => {
                let _: MsgDebug = x;
                log::debug!("Received MSG_DEBUG: {:?}", x.message);
                self.consume();
                return Ok(true);
            }
            None => (),
        }
        // Try to interpret as MSG_KEX_INIT. If successful, pass it to the kex handler.
        // Unless the protocol is violated, kex is in progress afterwards (if not already).
        match self.decode() {
            Some(msg) => {
                log::debug!("Received MSG_KEX_INIT");
                self.kex.push_init(msg)?;
                self.consume();
                return Ok(true);
            }
            None => (),
        }
        match self.decode() {
            Some(msg) => {
                log::debug!("Received MSG_ECDH_REPLY");
                self.kex.push_ecdh_reply(msg)?;
                self.consume();
                return Ok(true);
            }
            None => (),
        }
        match self.decode() {
            Some(msg) => {
                log::debug!("Received MSG_NEWKEYS");
                let _: MsgNewKeys = msg;
                let mut config = self.kex.push_new_keys()?;
                self.transmitter.decryption_ctx().update(
                    config.encryption_algorithm,
                    config.compression_algorithm,
                    config.mac_algorithm,
                    &mut config.key_streams.d(),
                );
                self.consume();
                return Ok(true);
            }
            None => (),
        }
        Ok(false)
    }

    pub fn poll_send_unimplemented(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<(), TransportError>> {
        let msg = MsgUnimplemented {
            packet_number: self.transmitter.packets_received() as u32,
        };
        let _ = self.transmitter.poll_send(cx, &msg);
        let _ = self.transmitter.poll_flush(cx);
        return Poll::Ready(Err(TransportError::MessageUnexpected));
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
