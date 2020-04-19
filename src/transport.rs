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

pub use self::buffered::*;
pub use self::config::*;
pub use self::error::*;
pub use self::identification::*;
pub use self::session_id::*;
pub use self::socket::*;

use self::cipher::*;
pub use self::kex::*;
use self::key_streams::*;
use self::msg_debug::*;
pub use self::msg_disconnect::*;
use self::msg_ignore::*;
use self::msg_service_accept::*;
use self::msg_service_request::*;
use self::msg_unimplemented::*;
use self::packet::*;
use self::transceiver::*;

use crate::codec::*;
use crate::host::*;

use async_std::future::poll_fn;
use async_std::net::TcpStream;
use async_std::task::{ready, Context, Poll};
use futures_timer::Delay;
use std::convert::From;
use std::option::Option;
use std::pin::Pin;
use std::sync::Arc;

pub trait TransportLayer: Send + Unpin + 'static {
    /// Try to decode the current message (only after `receive` or `poll_receive`).
    fn decode<Msg: Decode>(&mut self) -> Option<Msg>;

    /// Try to decode the current message (only after `receive` or `poll_receive`).
    ///
    /// In contrast to `decode` this method is able to decode messages that hold references into
    /// the receive buffer and may be used to avoid temporary heap allocations. Unfortunately,
    /// this borrows the transport reference which cannot be used until the message gets dropped.
    fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg>;

    /// Consumes the current message (only after `receive` or `poll_receive`).
    ///
    /// The message shall have been decoded and processed before being cosumed.
    fn consume(&mut self);

    /// Check whether the transport is flushed (output buffer empty).
    fn flushed(&self) -> bool;

    /// Try to flush all pending operations and output buffers.
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>>;

    /// Try to send a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if the output buffer is too full and must be flushed first.
    ///
    /// NB: Polling drives kex to completion.
    fn poll_send<M: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &M,
    ) -> Poll<Result<(), TransportError>>;

    /// Try to receive a message.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if no message is available for now.
    ///
    /// NB: Polling drives kex to completion.
    fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>>;

    /// Try to send MSG_DISCONNECT and swallow all errors.
    ///
    /// Message delivery may fail on errors or if output buffer is full.
    fn send_disconnect(&mut self, cx: &mut Context, reason: DisconnectReason);

    /// Try to send MSG_UNIMPLEMENTED and swallow all errors.
    ///
    /// Message delivery may fail on errors or if output buffer is full.
    fn send_unimplemented(&mut self, cx: &mut Context);

    /// Return the connection's session id.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    fn session_id(&self) -> &SessionId;
}

pub struct TransportLayerExt {}

impl TransportLayerExt {
    /// Send a message.
    pub async fn send<T: TransportLayer, M: Encode>(
        t: &mut T,
        msg: &M,
    ) -> Result<(), TransportError> {
        poll_fn(|cx| t.poll_send(cx, msg)).await
    }

    /// Receive a message.
    pub async fn receive<T: TransportLayer>(t: &mut T) -> Result<(), TransportError> {
        poll_fn(|cx| t.poll_receive(cx)).await
    }

    /// Flush the transport.
    pub async fn flush<T: TransportLayer>(t: &mut T) -> Result<(), TransportError> {
        poll_fn(|cx| t.poll_flush(cx)).await
    }

    /// Request a service by name.
    ///
    /// Service requests either succeeed or the connection is terminated by a disconnect message.
    pub async fn request_service<T: TransportLayer>(
        t: T,
        service_name: &str,
    ) -> Result<T, TransportError> {
        let mut t = t;
        let msg = MsgServiceRequest(service_name);
        Self::send(&mut t, &msg).await?;
        log::debug!("Sent MSG_SERVICE_REQUEST");
        Self::flush(&mut t).await?;
        Self::receive(&mut t).await?;
        let _: MsgServiceAccept<'_> = t.decode_ref().ok_or(TransportError::MessageUnexpected)?;
        t.consume();
        Ok(t)
    }
}

/// Implements the transport layer as described in RFC 4253.
///
/// This structure is polymorphic in role (either client or server) and the socket type (most
/// likely `TcpStream` but other types are used for testing).
/// The only difference between client and server is the key exchange mechanism and the
/// implementation is chosen at compile time dependant on the role parameter.
pub struct Transport<S: Socket = TcpStream> {
    trx: Transceiver<S>,
    kex: ClientKex,
}

impl<S: Socket> Transport<S> {
    /// Create a new transport acting as client.
    ///
    /// The initial key exchange has been completed successfully when function returns.
    pub async fn connect(
        config: &Arc<TransportConfig>,
        verifier: &Arc<dyn HostKeyVerifier>,
        hostname: String,
        socket: S,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(config, socket);
        trx.send_id(&config.identification).await?;
        let id = trx.receive_id().await?;
        let kex = ClientKex::new(&config, &verifier, id, hostname);
        let mut transport = Self { trx, kex };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Create a new transport acting as server.
    ///
    /// The initial key exchange has been completed successfully when function returns.
    pub async fn accept(
        _config: &Arc<TransportConfig>,
        _socket: S,
    ) -> Result<Self, TransportError> {
        unimplemented!()
    }

    /// Initiate a rekeying and wait for it to complete.
    async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex
            .init(self.trx.bytes_sent(), self.trx.bytes_received());
        poll_fn(|cx| {
            while self.kex.is_active() {
                ready!(self.poll_kex(cx))?;
                ready!(self.trx.poll_receive(cx))?;
                if !ready!(self.poll_consume_transport_message(cx))? {
                    self.send_unimplemented(cx);
                    return Poll::Ready(Err(TransportError::MessageUnexpected));
                }
            }
            Poll::Ready(Ok(()))
        })
        .await
    }

    /// This function is Ready unless sending an eventual kex message blocks.
    fn poll_kex(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let k = &mut self.kex;
        let t = &mut self.trx;
        if let Poll::Ready(x) = k.poll_init(cx, t.bytes_sent(), t.bytes_received()) {
            ready!(t.poll_send(cx, &x?))?;
            log::debug!("Sent MSG_KEX_INIT");
            k.push_init_tx()?;
        }
        if let Poll::Ready(x) = k.poll_ecdh_init(cx) {
            ready!(t.poll_send(cx, &x?))?;
            log::debug!("Sent MSG_ECDH_INIT");
            k.push_ecdh_init_tx()?;
        }
        if let Poll::Ready(x) = k.poll_ecdh_reply(cx) {
            ready!(t.poll_send(cx, &x?))?;
            log::debug!("Sent MSG_ECDH_REPLY");
            k.push_ecdh_reply_tx()?;
        }
        if let Poll::Ready(x) = k.poll_new_keys_tx(cx) {
            let enc = x?;
            ready!(t.poll_send(cx, &MsgNewKeys {}))?;
            log::debug!("Sent MSG_NEWKEYS");
            t.encryption_ctx()
                .update(enc.clone()) // FIXME clone
                .ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
            k.push_new_keys_tx()?;
        }
        t.poll_flush(cx)
    }

    /// Consumes message and returns true iff it is a transport message.
    fn poll_consume_transport_message(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<bool, TransportError>> {
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
                return Poll::Ready(Ok(true));
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
                return Poll::Ready(Ok(true));
            }
            None => (),
        }
        // Try to interpret as MSG_KEX_INIT. If successful, pass it to the kex handler.
        // Unless the protocol is violated, kex is in progress afterwards (if not already).
        match self.decode() {
            Some(msg) => {
                log::debug!("Received MSG_KEX_INIT");
                let tx = self.trx.bytes_sent();
                let rx = self.trx.bytes_received();
                self.kex.push_init_rx(tx, rx, msg)?;
                self.consume();
                return Poll::Ready(Ok(true));
            }
            None => (),
        }
        match self.decode() {
            Some(msg) => {
                log::debug!("Received MSG_ECDH_REPLY");
                self.kex.push_ecdh_reply_rx(msg)?;
                self.consume();
                return Poll::Ready(Ok(true));
            }
            None => (),
        }
        match self.decode() {
            Some(msg) => {
                let _: MsgNewKeys = msg;
                let dec = ready!(self.kex.poll_new_keys_rx(cx))?;
                let r = self.trx.decryption_ctx().update(dec);
                r.ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                self.kex.push_new_keys_rx()?;
                self.consume();
                log::debug!("Received MSG_NEWKEYS");
                return Poll::Ready(Ok(true));
            }
            None => (),
        }
        return Poll::Ready(Ok(false));
    }
}

impl<S: Socket> TransportLayer for Transport<S> {
    fn flushed(&self) -> bool {
        self.trx.flushed()
    }

    fn decode<Msg: Decode>(&mut self) -> Option<Msg> {
        self.trx.decode()
    }

    fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        self.trx.decode()
    }

    fn consume(&mut self) {
        self.trx.consume()
    }

    fn poll_send<M: Encode>(
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
                if ready!(self.poll_consume_transport_message(cx))? {
                    continue;
                } else {
                    self.send_unimplemented(cx);
                    return Poll::Ready(Err(TransportError::MessageUnexpected));
                }
            }
            return self.trx.poll_send(cx, &msg);
        }
    }

    fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
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
            if ready!(self.poll_consume_transport_message(cx))? {
                continue;
            }
            if self.kex.is_receiving_critical() {
                self.send_unimplemented(cx);
                return Poll::Ready(Err(TransportError::MessageUnexpected));
            }
            return Poll::Ready(Ok(()));
        }
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.trx.poll_inactivity(cx))?;
        self.trx.poll_flush(cx)
    }

    fn send_disconnect(&mut self, cx: &mut Context, reason: DisconnectReason) {
        let msg = MsgDisconnect::new(reason);
        let _ = self.trx.poll_send(cx, &msg);
        let _ = self.trx.poll_flush(cx);
    }

    fn send_unimplemented(&mut self, cx: &mut Context) {
        let msg = MsgUnimplemented {
            packet_number: self.trx.packets_received() as u32,
        };
        let _ = self.trx.poll_send(cx, &msg);
        let _ = self.trx.poll_flush(cx);
    }

    fn session_id(&self) -> &SessionId {
        &self.kex.session_id()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::sync::Mutex;

    pub struct TestTransport(Arc<Mutex<TestTransportState>>);
    pub struct TestTransportState {
        pub send_count: usize,
        pub receive_count: usize,
        pub consume_count: usize,
        pub flush_count: usize,
    }

    impl TestTransport {
        pub fn new() -> Self {
            Self(Arc::new(Mutex::new(TestTransportState {
                send_count: 0,
                receive_count: 0,
                consume_count: 0,
                flush_count: 0,
            })))
        }
    }

    impl TestTransport {
        pub fn send_count(&self) -> usize {
            (self.0).lock().unwrap().send_count
        }
        pub fn receive_count(&self) -> usize {
            (self.0).lock().unwrap().receive_count
        }
        pub fn consume_count(&self) -> usize {
            (self.0).lock().unwrap().consume_count
        }
        pub fn flush_count(&self) -> usize {
            (self.0).lock().unwrap().flush_count
        }
    }

    impl TransportLayer for TestTransport {
        fn decode<Msg: Decode>(&mut self) -> Option<Msg> {
            todo!("decode")
        }
        fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
            todo!("decode_ref")
        }
        fn consume(&mut self) {
            let mut x = (self.0).lock().unwrap();
            x.consume_count += 1;
        }
        fn flushed(&self) -> bool {
            todo!("flushed")
        }
        fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
            let mut x = (self.0).lock().unwrap();
            x.flush_count += 1;
            Poll::Ready(Ok(()))
        }
        fn poll_send<M: Encode>(
            &mut self,
            cx: &mut Context,
            msg: &M,
        ) -> Poll<Result<(), TransportError>> {
            let mut x = (self.0).lock().unwrap();
            x.send_count += 1;
            Poll::Pending
        }
        fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
            let mut x = (self.0).lock().unwrap();
            x.receive_count += 1;
            Poll::Pending
        }
        fn send_disconnect(&mut self, cx: &mut Context, reason: DisconnectReason) {
            todo!("send_disconnect")
        }
        fn send_unimplemented(&mut self, cx: &mut Context) {
            todo!("send_unimplemented")
        }
        fn session_id(&self) -> &SessionId {
            todo!("session_id")
        }
    }
}
