mod encryption;
mod error;
mod buffered_receiver;
mod buffered_sender;
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

pub use self::encryption::*;
pub use self::error::*;
pub use self::buffered_receiver::*;
pub use self::buffered_sender::*;
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
use futures::future::Future;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadHalf, WriteHalf};
use futures::ready;
use futures::stream::{Stream};
use futures::task::Context;
use futures::task::Poll;
use log;
use std::convert::From;
use std::marker::Unpin;
use std::option::Option;
use std::pin::Pin;
use std::time::{Duration, Instant};

pub enum Role {
    Client,
    Server,
}

#[derive(Debug)]
pub struct Token {
    packet_counter: u64,
    buffer_size: usize,
}

pub trait TransportStream:
    Read + AsyncRead + AsyncReadExt + Write + AsyncWrite + Unpin + Send + 'static
{
}

impl TransportStream for TcpStream {}

pub struct Transport<T> {
    role: Role,
    sender: BufferedSender<WriteHalf<T>>,
    receiver: BufferedReceiver<ReadHalf<T>>,
    local_id: Identification,
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
    unresolved_token: bool,
}

pub struct Receiver {}

impl<T: TransportStream> Transport<T> {
    const REKEY_BYTES: u64 = 1000_000_000;
    const REKEY_INTERVAL: Duration = Duration::from_secs(3600);

    const MAX_BUFFER_SIZE: usize = 35_000;

    pub async fn new(stream: T, role: Role) -> TransportResult<Self> {
        let (rh, wh) = stream.split();
        let mut sender = BufferedSender::new(wh);
        let mut receiver = BufferedReceiver::new(rh);

        let local_id = Self::send_id(&mut sender, Identification::default()).await?;
        let remote_id = Self::receive_id(&mut receiver).await?;

        let mut t = Transport {
            role: role,
            sender,
            receiver,
            local_id: local_id,
            remote_id: remote_id,
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
            unresolved_token: false,
        };

        t.kex(None).await?;
        Ok(t)
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub async fn send<M: Encode>(&mut self, msg: &M) -> TransportResult<()> {
        self.rekey_if_necessary().await?;
        self.send_raw(msg).await
    }

    pub async fn receive<'a, M: Decode<'a>>(&'a mut self) -> TransportResult<M> {
        self.rekey_if_necessary().await?;
        self.receive_raw().await
    }

    pub async fn try_receive<'a, M: Decode<'a>>(&'a mut self) -> Result<Option<M>, TransportError> {
        let x: E3<MsgDisconnect, MsgIgnore<'a>, M> = self.receive().await?;
        match x {
            E3::A(_) => Err(TransportError::DisconnectError),
            E3::B(_) => Ok(None),
            E3::C(x) => Ok(Some(x)),
        }
    }

    pub async fn flush(&mut self) -> TransportResult<()> {
        Ok(self.sender.flush().await?)
    }

    pub async fn rekey(&mut self) -> TransportResult<()> {
        self.kex(None).await
    }

    pub async fn request_service(mut self, service_name: &str) -> Result<Self, TransportError> {
        let req = MsgServiceRequest(service_name);
        self.send_raw(&req).await?;
        self.flush().await?;
        let _: MsgServiceAccept<'_> = self.receive_raw().await?;
        Ok(self)
    }

    async fn rekey_if_necessary(&mut self) -> TransportResult<()> {
        let bytes_sent_since = self.bytes_sent - self.kex_last_bytes_sent;
        let bytes_received_since = self.bytes_received - self.kex_last_bytes_received;
        if self.kex_last_time.elapsed() > Self::REKEY_INTERVAL
            || bytes_sent_since > Self::REKEY_BYTES
            || bytes_received_since > Self::REKEY_BYTES
        {
            self.rekey().await?
        }
        Ok(())
    }

    async fn kex(&mut self, remote: Option<KexInit>) -> TransportResult<()> {
        log::debug!("kex start");
        let local_init = KexInit::new(KexCookie::random());
        self.send_raw(&local_init).await?;
        self.flush().await?;
        let remote_init: KexInit = match remote {
            None => self.receive_raw().await?,
            Some(init) => init,
        };
        log::debug!("kex foo");

        let sid = match self.role {
            Role::Client => {
                let ecdh: Ecdh<X25519> = Ecdh::new(local_init, remote_init)?;

                self.send_raw(ecdh.init()).await?;
                self.flush().await?;
                let mut output = ecdh.reply(
                    self.receive_raw().await?,
                    &self.local_id,
                    &self.remote_id,
                    &self.session_id,
                )?;

                self.send_raw(&NewKeys {}).await?;
                self.flush().await?;
                let NewKeys {} = self.receive_raw().await?;

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

    async fn send_id(
        stream: &mut BufferedSender<WriteHalf<T>>,
        id: Identification,
    ) -> TransportResult<Identification> {
        let mut enc = BEncoder::from(stream.alloc(Encode::size(&id) + 2).await?);
        Encode::encode(&id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        stream.flush().await?;
        Ok(id)
    }

    async fn receive_id(
        stream: &mut BufferedReceiver<ReadHalf<T>>,
    ) -> TransportResult<Identification> {
        // Drop lines until remote SSH-2.0- version string is recognized
        loop {
            let line = stream.read_line(Identification::MAX_LEN).await?;
            match Decode::decode(&mut BDecoder(line)) {
                Some(id) => break Ok(id),
                None => (),
            }
        }
    }

    async fn send_raw<M: Encode>(&mut self, msg: &M) -> TransportResult<()> {
        let pc = self.packets_sent;
        self.packets_sent += 1;
        let layout = self.encryption_ctx.buffer_layout(Encode::size(msg));
        let buffer = self.sender.alloc(layout.buffer_len()).await?;
        let mut encoder = BEncoder::from(&mut buffer[layout.payload_range()]);
        Encode::encode(msg, &mut encoder);
        Ok(self.encryption_ctx.encrypt_packet(pc, layout, buffer))
    }

    pub fn send2<M: Encode>(&mut self, msg: &M) -> Option<()> {
        let layout = self.encryption_ctx.buffer_layout(Encode::size(msg));
        let buffer = self.sender.reserve(layout.buffer_len())?;
        let mut encoder = BEncoder::from(&mut buffer[layout.payload_range()]);
        Encode::encode(msg, &mut encoder);
        let pc = self.packets_sent;
        self.packets_sent += 1;
        self.encryption_ctx.encrypt_packet(pc, layout, buffer);
        Some(())
    }

    pub fn flush2(self) -> TransportFuture<T> {
        TransportFuture::flush(self)
    }

    async fn receive_raw<'a, M: Decode<'a>>(&'a mut self) -> TransportResult<M> {
        let pc = self.packets_received;
        log::error!("RECEIVE RAW {}", pc);
        self.packets_received += 1;
        self.receiver
            .fetch(2 * PacketLayout::PACKET_LEN_SIZE)
            .await?; // Don't decode size before 8 bytes have arrived

        let len: &[u8] = self.receiver.peek_exact(4).await?;
        let buffer_size = self
            .decryption_ctx
            .decrypt_buffer_size(pc, len)
            .filter(|size| *size <= Self::MAX_BUFFER_SIZE)
            .ok_or(TransportError::BadPacketLength)?;

        let buffer = self.receiver.read_exact(buffer_size).await?;
        let packet = self
            .decryption_ctx
            .decrypt_packet(pc, buffer)
            .ok_or(TransportError::MessageIntegrity)?;

        log::warn!("RECEIVE RAW {}", pc);

        Decode::decode(&mut BDecoder(&packet[1..])).ok_or(TransportError::DecoderError)
    }

    pub fn redeem_token<'a, M>(&'a mut self, token: Token) -> Option<M>
    where
        M: Decode<'a>,
    {
        assert!(self.unresolved_token);
        assert!(self.packets_received == token.packet_counter);

        self.packets_received += 1;
        self.unresolved_token = false;

        let buf = self.receiver.consume(token.buffer_size);
        log::error!("MESSAGE {:?}", &buf[5..]);
        Decode::decode(&mut BDecoder(&buf[5..]))
    }

    pub fn future(self) -> TransportFuture<T> {
        TransportFuture::ready(self)
    }
    /*
        pub fn for_each<E, H, F, O>(
            self,
            events: E,
            handler: H,
        ) -> ForEach<T, E, H, F, O>
        where
            E: Unpin + Stream + StreamExt,
            H: Unpin + FnMut(Self, Either<Token, E::Item>) -> F,
            F: Unpin + Future<Output = Result<Either<Transport<T>, O>, TransportError>>,
        {
            ForEach::new(self, events, handler)
        }
    */
}

pub enum TransportFuture<T> {
    Pending,
    Ready(Transport<T>),
    Flush(Transport<T>),
}

impl<T> TransportFuture<T> {
    pub fn ready(t: Transport<T>) -> Self {
        Self::Ready(t)
    }
    pub fn flush(t: Transport<T>) -> Self {
        Self::Flush(t)
    }
}

impl<T> Future for TransportFuture<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Output = Result<Transport<T>, TransportError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = Pin::into_inner(self);
        let x = std::mem::replace(s, TransportFuture::Pending);
        match x {
            Self::Pending => Poll::Pending,
            Self::Ready(t) => Poll::Ready(Ok(t)),
            Self::Flush(mut t) => match Pin::new(&mut t.sender).poll_flush(cx) {
                Poll::Pending => {
                    std::mem::replace(s, Self::Flush(t));
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => return Poll::Ready(Ok(t)),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
            },
        }
    }
}

impl<T: TransportStream> Stream for Transport<T> {
    type Item = Result<Token, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>
    where
        Self: Unpin,
    {
        assert!(!self.unresolved_token);
        let s = Pin::into_inner(self);
        let pc = s.packets_received;
        let mut r = Pin::new(&mut s.receiver);
        // Receive at least 8 bytes instead of the required 4
        // in order to impede traffic analysis (as recommended by RFC).
        match ready!(r.as_mut().poll_fetch(cx, 2 * PacketLayout::PACKET_LEN_SIZE)) {
            Ok(()) => (),
            Err(e) => return Poll::Ready(Some(Err(e.into()))),
        }
        // Decrypt the buffer len. Leave the original packet len field encrypted
        // in order to keep this function reentrant.
        assert!(r.window().len() >= PacketLayout::PACKET_LEN_SIZE);
        let mut len = [0; 4];
        len.copy_from_slice(&r.window()[..PacketLayout::PACKET_LEN_SIZE]);
        let len = s.decryption_ctx.decrypt_len(pc, len);
        if len > PacketLayout::MAX_PACKET_LEN {
            return Poll::Ready(Some(Err(TransportError::BadPacketLength)));
        }
        // Wait for the whole packet to arrive (including MAC etc)
        match ready!(r.as_mut().poll_fetch(cx, len)) {
            Err(e) => return Poll::Ready(Some(Err(e.into()))),
            Ok(()) => (),
        }
        // Try to decrypt the packet.
        assert!(r.window().len() >= len);
        match s
            .decryption_ctx
            .decrypt_packet(pc, &mut r.window_mut()[..len])
        {
            None => Poll::Ready(Some(Err(TransportError::MessageIntegrity))),
            Some(_) => {
                // Return a token containing the current packet counter
                // (for token uniqueness) and the number of bytes to be consumed
                // from the buffer when redeeming the token.
                s.unresolved_token = true;
                Poll::Ready(Some(Ok(Token {
                    packet_counter: pc,
                    buffer_size: len,
                })))
            }
        }
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
