pub(crate) mod config;
pub(crate) mod cookie;
pub(crate) mod crypto;
pub(crate) mod default;
pub(crate) mod ecdh_algorithm;
pub(crate) mod ecdh_hash;
pub(crate) mod error;
pub(crate) mod identification;
pub(crate) mod kex;
pub(crate) mod key_streams;
pub(crate) mod message;
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
pub(crate) mod service;
pub(crate) mod session_id;
pub(crate) mod transceiver;

pub use self::config::*;
pub use self::default::*;
pub use self::error::*;
pub use self::identification::*;
pub use self::message::*;
pub use self::service::*;
pub use self::session_id::*;

pub use self::crypto::*;
pub use self::kex::*;
use self::key_streams::*;
use self::msg_debug::*;
pub use self::msg_disconnect::*;
use self::msg_ignore::*;
use self::msg_service_accept::*;
use self::msg_service_request::*;
use self::msg_unimplemented::*;
use self::transceiver::*;

use crate::auth::Agent;
use crate::known_hosts::*;
use crate::util::codec::*;
use crate::util::socket::*;

use async_std::future::poll_fn;
use async_std::net::TcpStream;
use async_std::task::{ready, Context, Poll};
use std::convert::From;
use std::fmt::Debug;
use std::option::Option;
use std::pin::Pin;
use std::sync::Arc;

pub const PAYLOAD_MAX_LEN: usize = 32_768;
pub const PADDING_MIN_LEN: usize = 4;
pub const PADDING_LEN_BYTES: usize = 1;
pub const PACKET_MIN_LEN: usize = 16;
pub const PACKET_MAX_LEN: usize = 35_000;
pub const PACKET_LEN_BYTES: usize = 4;

pub trait Transport: Debug + Send + Unpin + 'static {
    /// Try to receive the next message buffer.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if no message is available for now.
    ///
    /// NB: Polling drives kex to completion.
    fn poll_peek(&mut self, cx: &mut Context) -> Poll<Result<&[u8], TransportError>>;

    /// Consumed the current rx buffer (only after `rx_peek`).
    ///
    /// The message shall have been decoded and processed before being cosumed.
    fn consume(&mut self);

    /// Try to reserve buffer space for the next message to send.
    ///
    /// Returns `Pending` if any internal process (like kex) is in a critical stage that must not
    /// be interrupted or if the output buffer is too full and must be flushed first.
    ///
    /// NB: Polling drives kex to completion.
    fn poll_alloc(&mut self, cx: &mut Context, len: usize) -> Poll<Result<&mut [u8], TransportError>>;

    /// Commits the current tx buffer as ready for sending (only after `tx_alloc`).
    ///
    /// A message shall have been written to the tx buffer.
    fn commit(&mut self);

    /// Try to flush all pending operations and output buffers.
    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>>;

    /// Try to send MSG_DISCONNECT and swallow all errors.
    ///
    /// Message delivery may silently fail on errors or if output buffer is full.
    fn send_disconnect(&mut self, cx: &mut Context, reason: DisconnectReason);

    /// Try to send MSG_UNIMPLEMENTED and swallow all errors.
    ///
    /// Message delivery may silently fail on errors or if output buffer is full.
    fn send_unimplemented(&mut self, cx: &mut Context);

    /// Return the connection's session id.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    fn session_id(&self) -> Result<&SessionId, TransportError>;
}

pub struct TransportExt {}

impl TransportExt {
    /// Send a message.
    pub async fn send<M: Encode>(
        t: &mut Box<dyn Transport>,
        msg: &M,
    ) -> Result<(), TransportError> {
        poll_fn(|cx| Self::poll_send(t, cx, msg)).await
    }

    /// Receive a message.
    pub async fn receive<M: Decode>(t: &mut Box<dyn Transport>) -> Result<M, TransportError> {
        poll_fn(|cx| {
            let rx = ready!(t.poll_peek(cx))?;
            let msg = SliceDecoder::decode(rx).ok_or(TransportError::DecoderError)?;
            t.consume();
            Poll::Ready(Ok(msg))
        })
        .await
    }

    /// Flush the transport.
    pub async fn flush(t: &mut Box<dyn Transport>) -> Result<(), TransportError> {
        poll_fn(|cx| t.poll_flush(cx)).await
    }

    /// Request a service by name.
    ///
    /// Service requests either succeeed or the connection is terminated by a disconnect message.
    pub async fn request_service(
        t: Box<dyn Transport>,
        service_name: &str,
    ) -> Result<Box<dyn Transport>, TransportError> {
        let mut t = t;
        let msg = MsgServiceRequest(service_name);
        Self::send(&mut t, &msg).await?;
        log::debug!("Sent MSG_SERVICE_REQUEST");
        Self::flush(&mut t).await?;
        Self::receive::<MsgServiceAccept>(&mut t).await?;
        log::debug!("Received MSG_SERVICE_ACCEPT");
        Ok(t)
    }

    pub async fn offer_service(
        _t: Box<dyn Transport>,
        _service_name: &str,
    ) -> Result<Box<dyn Transport>, TransportError> {
        todo!()
    }

    pub fn poll_send<M: Encode>(
        t: &mut Box<dyn Transport>,
        cx: &mut Context,
        msg: &M,
    ) -> Poll<Result<(), TransportError>> {
        let buf = ready!(t.poll_alloc(cx, msg.size()))?;
        SliceEncoder::encode_into(msg, buf);
        t.commit();
        Poll::Ready(Ok(()))
    }
}

/*
#[cfg(test)]
pub mod tests {
    use super::*;

    use std::collections::VecDeque;

    pub struct TestTransport {
        send_count: usize,
        receive_count: usize,
        consume_count: usize,
        flush_count: usize,
        rx_buf: VecDeque<Vec<u8>>,
        tx_buf: Vec<Vec<u8>>,
        tx_sent: Vec<Vec<Vec<u8>>>,
        tx_ready: bool,
        tx_disconnect: Option<DisconnectReason>,
        error: Option<TransportError>,
    }

    impl TestTransport {
        pub fn new() -> Self {
            Self {
                send_count: 0,
                receive_count: 0,
                consume_count: 0,
                flush_count: 0,
                rx_buf: VecDeque::new(),
                tx_buf: vec![],
                tx_sent: vec![],
                tx_ready: false,
                tx_disconnect: None,
                error: None,
            }
        }

        pub fn check_error(&self) -> Result<(), TransportError> {
            if let Some(e) = self.error {
                Err(e)
            } else {
                Ok(())
            }
        }
    }

    impl TestTransport {
        pub fn send_count(&self) -> usize {
            self.send_count
        }
        pub fn receive_count(&self) -> usize {
            self.receive_count
        }
        pub fn consume_count(&self) -> usize {
            self.consume_count
        }
        pub fn flush_count(&self) -> usize {
            self.flush_count
        }
        pub fn set_tx_ready(&mut self, ready: bool) {
            self.tx_ready = ready;
        }
        pub fn tx_buf(&self) -> Vec<Vec<u8>> {
            self.tx_buf.clone()
        }
        pub fn tx_sent(&self) -> Vec<Vec<Vec<u8>>> {
            self.tx_sent.clone()
        }
        pub fn tx_disconnect(&self) -> Option<DisconnectReason> {
            self.tx_disconnect
        }
        pub fn rx_push<E: Encode>(&mut self, msg: &E) {
            self.rx_buf.push_back(SliceEncoder::encode(msg))
        }
        pub fn set_error(&mut self, e: TransportError) {
            self.error = Some(e)
        }
    }

    impl Transport for TestTransport {
        fn rx_buffer(&self) -> Option<&[u8]> {
            panic!()
        }
        fn tx_buffer(&mut self) -> &mut [u8] {
            panic!()
        }

        fn decode<Msg: Decode>(&mut self) -> Option<Msg> {
            if let Some(data) = self.rx_buf.front() {
                SliceDecoder::decode(data)
            } else {
                None
            }
        }
        fn decode_ref<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
            if let Some(data) = self.rx_buf.front() {
                SliceDecoder::decode(data)
            } else {
                None
            }
        }
        fn consume(&mut self) {
            if self.rx_buf.pop_front().is_none() {
                panic!("consume called on empty rx_buf")
            }
        }
        fn flushed(&self) -> bool {
            todo!("flushed")
        }
        fn poll_flush(&mut self, _cx: &mut Context) -> Poll<Result<(), TransportError>> {
            self.flush_count += 1;
            self.check_error()?;
            if !self.tx_buf.is_empty() {
                let buf = std::mem::replace(&mut self.tx_buf, vec![]);
                self.tx_sent.push(buf);
                Poll::Ready(Ok(()))
            } else {
                Poll::Ready(Ok(()))
            }
        }
        fn poll_send<M: Encode>(
            &mut self,
            _cx: &mut Context,
            msg: &M,
        ) -> Poll<Result<(), TransportError>> {
            self.send_count += 1;
            self.check_error()?;
            if self.tx_ready {
                self.tx_buf.push(SliceEncoder::encode(msg));
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }
        fn poll_receive(&mut self, _cx: &mut Context) -> Poll<Result<(), TransportError>> {
            self.receive_count += 1;
            self.check_error()?;
            if !self.rx_buf.is_empty() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }
        fn send_disconnect(&mut self, _cx: &mut Context, reason: DisconnectReason) {
            if self.tx_ready {
                self.tx_disconnect = Some(reason);
            }
        }
        fn send_unimplemented(&mut self, _cx: &mut Context) {
            todo!("send_unimplemented")
        }
        fn session_id(&self) -> Result<&SessionId, TransportError> {
            todo!("session_id")
        }
    }
}
*/
