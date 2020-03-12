pub(crate) mod buffer;
pub(crate) mod buffered;
pub(crate) mod cipher;
pub(crate) mod config;
pub(crate) mod cookie;
pub(crate) mod ecdh_algorithm;
pub(crate) mod ecdh_hash;
pub(crate) mod error;
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
pub(crate) mod transceiver;

pub use self::buffer::*;
pub use self::buffered::*;
pub use self::config::*;
pub use self::error::*;
pub use self::identification::*;
pub use self::session_id::*;
pub use self::socket::*;

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
use self::transceiver::*;

use crate::client::Client;
use crate::codec::*;
use crate::host::*;
use crate::role::*;
use crate::server::Server;

use async_std::future::poll_fn;
use async_std::net::TcpStream;
use async_std::task::{ready, Context, Poll};
use futures_timer::Delay;
use std::convert::From;
use std::option::Option;
use std::pin::Pin;
use std::sync::Arc;

/// Implements the transport layer as described in RFC 4253.
///
/// This structure is polymorphic in role (either client or server) and the socket type (most
/// likely `TcpStream` but other types are used for testing).
/// The only difference between client and server is the key exchange mechanism and the
/// implementation is chosen at compile time dependant on the role parameter.
pub struct Transport<R: Role, S: Socket = TcpStream> {
    trx: Transceiver<S>,
    kex: <R as Role>::Kex,
}

impl<S: Socket> Transport<Client, S> {
    /// Create a new transport.
    ///
    /// The initial key exchange has been completed successfully when this
    /// function does not return an error.
    pub async fn new<C: TransportConfig>(
        config: &C,
        verifier: Arc<Box<dyn HostKeyVerifier>>,
        hostname: String,
        socket: S,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(config, socket);
        trx.send_id(config.identification()).await?;
        let id = trx.receive_id().await?;
        let kex = ClientKex::new(config, verifier.clone(), id, hostname);
        let mut transport = Self { trx, kex };
        transport.rekey().await?;
        Ok(transport)
    }
}

impl<S: Socket> Transport<Server, S> {
    /// Create a new transport.
    ///
    /// The initial key exchange has been completed successfully when this
    /// function does not return an error.
    pub async fn new<C: TransportConfig>(_config: &C, _socket: S) -> Result<Self, TransportError> {
        unimplemented!()
    }
}

impl<R: Role, S: Socket> Transport<R, S> {
    /// Return the connection's session id.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    pub fn session_id(&self) -> &SessionId {
        &self.kex.session_id()
    }

    /// Check whether the transport is flushed (output buffer empty).
    pub fn flushed(&self) -> bool {
        self.trx.flushed()
    }

    /// Initiate a rekeying and wait for it to complete.
    pub async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex.init();
        poll_fn(|cx| {
            while self.kex.is_active() {
                ready!(self.poll_kex(cx))?;
                ready!(self.trx.poll_receive(cx))?;
                if !self.consume_transport_message()? {
                    return self.poll_send_unimplemented(cx);
                }
            }
            Poll::Ready(Ok(()))
        })
        .await
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
        self.trx.decode()
    }

    /// Try to decode the current message (only after `receive` or `poll_receive`).
    ///
    /// In contrast to `decode` this method is able to decode messages that hold references into
    /// the receive buffer and may be used to avoid temporary heap allocations. Unfortunately,
    /// this borrows the transport reference which cannot be used until the message gets dropped.
    pub fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        self.trx.decode()
    }

    /// Consumes the current message (only after `receive` or `poll_receive`).
    ///
    /// The message shall have been decoded and processed before being cosumed.
    pub fn consume(&mut self) {
        self.trx.consume()
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
        // The next call does nothing unless the keep-alive timeout triggers.
        // When it triggers, it tries to send a message ignore which might pend.
        ready!(self.trx.poll_keepalive(cx))?;
        // The next call does nothing unless the remote inactivity timeout triggers.
        // When it triggers, it returns an error.
        ready!(self.trx.poll_inactivity(cx))?;
        // In case a running kex forbids sending no-kex packets we need to drive
        // kex to completion first. This requires dispatching transport messages.
        // It might happen that kex can be completed non-blocking and sending the
        // message migh succeed in a later loop iteration.
        loop {
            ready!(self.poll_kex(cx))?;
            if self.kex.is_sending_critical() {
                ready!(self.trx.poll_receive(cx))?;
                if self.consume_transport_message()? {
                    continue;
                } else {
                    return self.poll_send_unimplemented(cx);
                }
            }
            return self.trx.poll_send(cx, &msg);
        }
    }

    /// Try to receive a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if no message is available for now.
    ///
    /// NB: Polling drives kex to completion.
    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        // See `poll_send` concerning the next two calls.
        ready!(self.trx.poll_keepalive(cx))?;
        ready!(self.trx.poll_inactivity(cx))?;
        // Transport messages are handled internally by this function. In such a case the loop
        // will iterate more than once but always terminate with either Ready or Pending.
        // In case a running kex forbids receiving non-kex packets we need to drive kex to
        // completion first: This means dispatching transport messages only; all other packets
        // will cause an error.
        loop {
            ready!(self.poll_kex(cx))?;
            ready!(self.trx.poll_receive(cx))?;
            if self.consume_transport_message()? {
                continue;
            }
            if self.kex.is_receiving_critical() {
                return self.poll_send_unimplemented(cx);
            }
            return Poll::Ready(Ok(()));
        }
    }

    /// Try to flush all pending operations and output buffers.
    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.trx.poll_inactivity(cx))?;
        self.trx.poll_flush(cx)
    }

    /// Try to send MSG_UNIMPLEMENTED and return `MessageUnexpected` error
    /// even in the presence of other errors (preserve the former).
    /// Does not block even if the message cannot be sent.
    pub fn poll_send_unimplemented(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<(), TransportError>> {
        let msg = MsgUnimplemented {
            packet_number: self.trx.packets_received() as u32,
        };
        let _ = self.trx.poll_send(cx, &msg);
        let _ = self.trx.poll_flush(cx);
        Poll::Ready(Err(TransportError::MessageUnexpected))
    }

    /// This function is Ready unless sending an eventual kex message blocks.
    fn poll_kex(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let k = &mut self.kex;
        let t = &mut self.trx;
        // `poll` determines whether a kex is necessary or in progress and uses the lambda to send
        // as many messages as necessary. It returns Pending in case sending blocks. The next
        // invocation of this function will then proceed right where it blocked before.
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
                KexOutput::NewKeys(enc) => {
                    ready!(t.poll_send(cx, &MsgNewKeys {}))?;
                    log::debug!("Sent MSG_NEWKEYS");
                    t.encryption_ctx()
                        .update(enc)
                        .ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                }
            }
            t.poll_flush(cx)
        })
    }

    /// Consumes message and returns true iff it is a transport message.
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
                let sent = self.trx.bytes_sent();
                let rcvd = self.trx.bytes_received();
                let dec = self.kex.push_new_keys(sent, rcvd)?;
                let r = self.trx.decryption_ctx().update(dec);
                r.ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                self.consume();
                return Ok(true);
            }
            None => (),
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    //use super::*;
}
