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

use crate::codec::*;

use async_std::io::{Read, Write};
use async_std::net::TcpStream;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadHalf, WriteHalf};
use futures::ready;
use futures::task::Context;
use futures::task::Poll;
use log;
use std::convert::From;
use std::marker::Unpin;
use std::option::Option;
use std::pin::Pin;
use std::time::Instant;

pub enum Role {
    Client,
    Server,
}

pub trait TransportStream:
    Read + AsyncRead + AsyncReadExt + Write + AsyncWrite + Unpin + Send + 'static
{
}

pub struct TransportConfig {
    identification: Identification,
    rekey_bytes: u64,
    rekey_interval: std::time::Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            identification: Identification::default(),
            rekey_bytes: 1_000_000_000,
            rekey_interval: std::time::Duration::from_secs(3600),
        }
    }
}

impl TransportStream for TcpStream {}

pub struct Transport<T> {
    config: TransportConfig,
    role: Role,
    sender: BufferedSender<WriteHalf<T>>,
    receiver: BufferedReceiver<ReadHalf<T>>,
    remote_id: Identification,
    session_id: SessionId,
    bytes_sent: u64,
    packets_sent: u64,
    bytes_received: u64,
    packets_received: u64,
    kex_last_time: Instant,
    kex_last_bytes_received: u64,
    kex_last_bytes_sent: u64,
    encryption_ctx: EncryptionContext,
    decryption_ctx: EncryptionContext,
    inbox: Option<usize>,
}

impl<T: TransportStream> Transport<T> {
    /// Create a new transport.
    ///
    /// The initial key exchange has been completed successfully when this
    /// function does not return an error.
    pub async fn new(
        config: TransportConfig,
        stream: T,
        role: Role,
    ) -> Result<Self, TransportError> {
        let (rh, wh) = stream.split();
        let mut sender = BufferedSender::new(wh);
        let mut receiver = BufferedReceiver::new(rh);

        Self::send_id(&mut sender, &config.identification).await?;
        let remote_id = Self::receive_id(&mut receiver).await?;

        let mut t = Transport {
            config,
            role,
            sender,
            receiver,
            remote_id,
            session_id: SessionId::None,
            bytes_sent: 0,
            packets_sent: 0,
            bytes_received: 0,
            packets_received: 0,
            kex_last_time: Instant::now(),
            kex_last_bytes_sent: 0,
            kex_last_bytes_received: 0,
            encryption_ctx: EncryptionContext::new(),
            decryption_ctx: EncryptionContext::new(),
            inbox: None,
        };

        t.kex(None).await?;
        Ok(t)
    }

    /// Return the session id belonging to the connection.
    ///
    /// The session id is a result of the initial key exchange. It is static for the whole
    /// lifetime of the connection.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// TODO
    pub async fn send<M: Encode>(&mut self, msg: &M) -> Result<(), TransportError> {
        self.rekey_if_necessary().await?;
        self.send_raw(msg).await
    }

    /// TODO
    pub async fn receive(&mut self) -> Result<(), TransportError> {
        //self.rekey_if_necessary().await?;
        self.receive_raw().await
    }

    /// Flush the transport.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        Ok(self.sender.flush().await?)
    }

    /// Check whether the transport is flushed.
    pub fn flushed(&self) -> bool {
        self.sender.flushed()
    }

    // TODO
    pub async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex(None).await
    }

    pub async fn request_service(mut self, service_name: &str) -> Result<Self, TransportError> {
        let req = MsgServiceRequest(service_name);
        self.send_raw(&req).await?;
        self.flush().await?;
        self.receive_raw().await?;
        let _: MsgServiceAccept<'_> = self.decode().unwrap(); // TODO
        self.consume();
        Ok(self)
    }

    async fn rekey_if_necessary(&mut self) -> Result<(), TransportError> {
        let bytes_sent_since = self.bytes_sent - self.kex_last_bytes_sent;
        let bytes_received_since = self.bytes_received - self.kex_last_bytes_received;
        if self.kex_last_time.elapsed() > self.config.rekey_interval
            || bytes_sent_since > self.config.rekey_bytes
            || bytes_received_since > self.config.rekey_bytes
        {
            self.rekey().await?
        }
        Ok(())
    }

    async fn kex(&mut self, remote: Option<KexInit>) -> Result<(), TransportError> {
        log::debug!("kex start");
        let local_init = KexInit::new(KexCookie::random());
        self.send_raw(&local_init).await?;
        self.flush().await?;
        let remote_init: KexInit = match remote {
            None => {
                self.receive_raw().await?;
                let x = self.decode().unwrap();
                self.consume();
                x
            }
            Some(init) => init,
        };
        log::debug!("kex foo");

        let sid = match self.role {
            Role::Client => {
                let ecdh: Ecdh<X25519> = Ecdh::new(local_init, remote_init)?;

                self.send_raw(ecdh.init()).await?;
                self.flush().await?;
                self.receive().await?;
                let mut output = ecdh.reply(
                    self.decode().unwrap(), // TODO
                    &self.config.identification,
                    &self.remote_id,
                    &self.session_id,
                )?;
                self.consume();

                self.send_raw(&NewKeys {}).await?;
                self.flush().await?;
                self.receive().await?;
                let NewKeys {} = self.decode().unwrap(); // TODO
                self.consume();

                self.encryption_ctx.new_keys(
                    &output.encryption_algorithm_client_to_server,
                    &output.compression_algorithm_client_to_server,
                    &output.mac_algorithm_client_to_server,
                    &mut output.key_streams.c(),
                );
                self.decryption_ctx.new_keys(
                    &output.encryption_algorithm_server_to_client,
                    &output.compression_algorithm_server_to_client,
                    &output.mac_algorithm_server_to_client,
                    &mut output.key_streams.d(),
                );

                output.session_id
            }
            Role::Server => panic!("server role not implemented yet"),
        };
        self.kex_last_time = Instant::now();
        self.kex_last_bytes_received = self.bytes_received;
        self.kex_last_bytes_sent = self.bytes_sent;
        // The session id will only be set after the initial key exchange
        sid.map(|s| self.session_id = s);
        log::debug!("kex complete");
        Ok(())
    }

    /// Send the local identification string.
    async fn send_id(
        stream: &mut BufferedSender<WriteHalf<T>>,
        id: &Identification,
    ) -> Result<(), TransportError> {
        let mut enc = BEncoder::from(stream.alloc(Encode::size(&id) + 2).await?);
        Encode::encode(&id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        stream.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    async fn receive_id(
        stream: &mut BufferedReceiver<ReadHalf<T>>,
    ) -> Result<Identification, TransportError> {
        // Drop lines until remote SSH-2.0- version string is recognized
        loop {
            let line = stream.read_line(Identification::MAX_LEN).await?;
            match DecodeRef::decode(&mut BDecoder(line)) {
                None => (),
                Some(id) => return Ok(id),
            }
        }
    }

    async fn send_raw<M: Encode>(&mut self, msg: &M) -> Result<(), TransportError> {
        futures::future::poll_fn(|cx| self.poll_send(cx, msg)).await
    }

    async fn receive_raw(&mut self) -> Result<(), TransportError> {
        futures::future::poll_fn(|cx| self.poll_receive(cx)).await
    }

    pub fn poll_send<Msg: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let layout = self.encryption_ctx.buffer_layout(Encode::size(msg));
        loop {
            match self.sender.reserve(layout.buffer_len()) {
                None => {
                    ready!(self.poll_flush(cx))?;
                    continue; // unlikely to succeed immediately
                }
                Some(buffer) => {
                    let mut encoder = BEncoder::from(&mut buffer[layout.payload_range()]);
                    Encode::encode(msg, &mut encoder);
                    let pc = self.packets_sent;
                    self.packets_sent += 1;
                    self.encryption_ctx.encrypt_packet(pc, layout, buffer);
                    return Poll::Ready(Ok(()));
                }
            }
        }
    }

    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        Pin::new(&mut (self.sender))
            .poll_flush(cx)
            .map(|x| x.map_err(Into::into))
    }

    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        if self.inbox.is_some() {
            log::error!("POLL READY");
            return Poll::Ready(Ok(()));
        }
        let s = self;
        let pc = s.packets_received;
        let mut r = Pin::new(&mut s.receiver);
        // Receive at least 8 bytes instead of the required 4
        // in order to impede traffic analysis (as recommended by RFC).
        match ready!(r.as_mut().poll_fetch(cx, 2 * PacketLayout::PACKET_LEN_SIZE)) {
            Ok(()) => (),
            Err(e) => return Poll::Ready(Err(e.into())),
        }
        // Decrypt the buffer len. Leave the original packet len field encrypted
        // in order to keep this function reentrant.
        assert!(r.window().len() >= PacketLayout::PACKET_LEN_SIZE);
        let mut len = [0; 4];
        len.copy_from_slice(&r.window()[..PacketLayout::PACKET_LEN_SIZE]);
        let len = s.decryption_ctx.decrypt_len(pc, len);
        if len > PacketLayout::MAX_PACKET_LEN {
            return Poll::Ready(Err(TransportError::BadPacketLength));
        }
        // Wait for the whole packet to arrive (including MAC etc)
        match ready!(r.as_mut().poll_fetch(cx, len)) {
            Err(e) => return Poll::Ready(Err(e.into())),
            Ok(()) => (),
        }
        // Try to decrypt the packet.
        assert!(r.window().len() >= len);
        match s
            .decryption_ctx
            .decrypt_packet(pc, &mut r.window_mut()[..len])
        {
            None => Poll::Ready(Err(TransportError::MessageIntegrity)),
            Some(_) => {
                s.inbox = Some(len);
                Poll::Ready(Ok(()))
            }
        }
    }

    pub fn decode<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        match self.inbox {
            None => panic!("nothing to decode"),
            Some(_) => {
                // TODO: Use layout
                DecodeRef::decode(&mut BDecoder(&self.receiver.window()[5..]))
            }
        }
    }

    pub fn consume(&mut self) {
        match self.inbox {
            None => panic!("nothing to consume"),
            Some(buffer_size) => {
                self.packets_received += 1;
                self.receiver.consume(buffer_size);
                self.inbox = None;
            }
        }
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
