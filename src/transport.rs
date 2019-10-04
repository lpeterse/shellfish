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

use crate::codec::*;

use async_std::io::{Read, Write};
use async_std::net::TcpStream;
use futures::future::poll_fn;
use futures::future::FutureExt;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadHalf, WriteHalf};
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use futures_timer::Delay;
use std::convert::From;
use std::marker::Unpin;
use std::option::Option;
use std::pin::Pin;

pub trait KexState {
    fn push_msg(&mut self, msg: &[u8]) -> Result<(), KexError>;
}

pub trait TransportStream:
    Read + AsyncRead + AsyncReadExt + Write + AsyncWrite + Unpin + Send + 'static
{
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
            kex_interval_duration: std::time::Duration::from_secs(3),
            alive_interval: std::time::Duration::from_secs(300),
            inactivity_timeout: std::time::Duration::from_secs(330),
        }
    }
}

impl TransportStream for TcpStream {}

pub struct Transport<T> {
    transmitter: Transmitter<T>,
    alive_timer: Delay,
    inactivity_timer: Delay,
    kex: ClientKexMachine,
}

impl<T: TransportStream> Transport<T> {
    /// Create a new transport.
    ///
    /// The initial key exchange has been completed successfully when this
    /// function does not return an error.
    pub async fn new(config: &TransportConfig, stream: T) -> Result<Self, TransportError> {
        let mut transport = Self {
            transmitter: Transmitter::new(stream, config.identification.clone()).await?,
            alive_timer: Delay::new(config.alive_interval),
            inactivity_timer: Delay::new(config.inactivity_timeout),
            kex: ClientKexMachine::new(config.kex_interval_bytes, config.kex_interval_duration),
        };
        transport.rekey().await?;
        Ok(transport)
    }

    pub async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex.init_local();
        poll_fn(|cx| self.poll_internal(cx)).await
    }

    /// Return the session id belonging to the connection.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    pub fn session_id(&self) -> &SessionId {
        &self.kex.session_id
    }

    /// TODO
    pub async fn send<M: Encode>(&mut self, msg: &M) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_send(cx, msg)).await
    }

    /// TODO
    pub async fn receive(&mut self) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_receive(cx)).await
    }

    /// Flush the transport.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        self.transmitter.flush().await
    }

    /// Check whether the transport is flushed.
    pub fn flushed(&self) -> bool {
        self.transmitter.flushed()
    }

    pub async fn request_service(mut self, service_name: &str) -> Result<Self, TransportError> {
        let req = MsgServiceRequest(service_name);
        self.send(&req).await?;
        self.flush().await?;
        self.receive().await?;
        let _: MsgServiceAccept<'_> = self.decode().unwrap(); // FIXME
        self.consume();
        Ok(self)
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

    pub fn decode<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        self.transmitter.decode()
    }

    pub fn decode2<Msg: Decode>(&mut self) -> Option<Msg> {
        self.transmitter.decode()
    }

    pub fn consume(&mut self) {
        self.transmitter.consume()
    }

    pub fn poll_internal(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        loop {
            if !self.kex.is_in_progress(cx, &mut self.transmitter)? {
                return Poll::Ready(Ok(()));
            }
            ready!(self.kex.poll_flush(cx, &mut self.transmitter))?;
            ready!(self.transmitter.poll_receive(cx))?;
            match self.decode() {
                Some(x) => {
                    let _: MsgDisconnect = x;
                    return Poll::Ready(Err(TransportError::DisconnectByPeer));
                }
                None => (),
            }
            match self.decode() {
                Some(x) => {
                    let _: MsgIgnore = x;
                    self.consume();
                    continue;
                }
                None => (),
            }
            match self.decode() {
                Some(x) => {
                    let _: MsgUnimplemented = x;
                    return Poll::Ready(Err(TransportError::MessageUnimplemented(x)));
                }
                None => (),
            }
            match self.decode() {
                Some(x) => {
                    let _: MsgDebug = x;
                    log::debug!("{:?}", x);
                    self.consume();
                    continue;
                }
                None => (),
            }
            match self.decode2() {
                Some(msg) => {
                    self.kex.init_remote(msg)?;
                    self.consume();
                    continue;
                }
                None => (),
            }
            if self.kex.is_init_received() {
                self.kex.consume(&mut self.transmitter)?;
                continue;
            }
            // Kex is in progress, but the KEX_INIT packet from
            // remote has not been received yet which means that other
            // packets may arrive before.
            return Poll::Ready(Ok(()));
        }
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
