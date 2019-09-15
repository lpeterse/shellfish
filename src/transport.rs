mod msg_debug;
mod msg_disconnect;
mod encryption;
mod error;
mod identification;
mod kex;
mod key_streams;
mod packet_layout;
mod session_id;
mod msg_ignore;
mod msg_unimplemented;
mod msg_service_request;
mod msg_service_accept;

pub use self::msg_debug::*;
pub use self::msg_disconnect::*;
pub use self::encryption::*;
pub use self::error::*;
pub use self::identification::*;
pub use self::kex::*;
pub use self::key_streams::*;
pub use self::packet_layout::*;
pub use self::session_id::*;
pub use self::msg_ignore::*;
pub use self::msg_unimplemented::*;
pub use self::msg_service_request::*;
pub use self::msg_service_accept::*;

use crate::buffer::*;
use crate::codec::*;

use async_std::io::{Read, Write};
use futures::io::{AsyncRead, AsyncWrite};
use std::convert::{From};
use std::time::{Duration, Instant};
use async_std::net::{TcpStream};
use log;

pub enum Role {
    Client,
    Server,
}

pub trait TransportStream: Read + AsyncRead + Unpin + Write + AsyncWrite + Unpin + Send + 'static {}

impl TransportStream for TcpStream {}

pub struct Transport<T> {
    role: Role,
    stream: Buffer<T>,
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
}

impl<T> Transport<T>
where
    T: Read + AsyncRead + Unpin,
    T: Write + AsyncWrite + Unpin,
{
    const REKEY_BYTES: u64 = 1000_000_000;
    const REKEY_INTERVAL: Duration = Duration::from_secs(3600);

    const MAX_BUFFER_SIZE: usize = 35_000;

    pub async fn new(stream: T, role: Role) -> TransportResult<Self> {
        let mut buffer = Buffer::new(stream);

        let local_id = Self::send_id(&mut buffer, Identification::default()).await?;
        let remote_id = Self::receive_id(&mut buffer).await?;

        let mut t = Transport {
            role: role,
            stream: buffer,
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

    pub async fn flush(&mut self) -> TransportResult<()> {
        Ok(self.stream.flush().await?)
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
        stream: &mut Buffer<T>,
        id: Identification,
    ) -> TransportResult<Identification> {
        let mut enc = BEncoder::from(stream.alloc(Encode::size(&id) + 2).await?);
        Encode::encode(&id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        stream.flush().await?;
        Ok(id)
    }

    async fn receive_id(stream: &mut Buffer<T>) -> TransportResult<Identification> {
        // Drop lines until remote SSH-2.0- version string is recognized
        loop {
            let line = stream.read_line(Identification::MAX_LEN).await?;
            match Decode::decode(&mut BDecoder(line)) {
                Some(id) => break Ok(id),
                None => (),
            }
        }
    }

    async fn send_raw<'a, M: Encode>(&mut self, msg: &M) -> TransportResult<()> {
        let pc = self.packets_sent;
        self.packets_sent += 1;
        let layout = self.encryption_ctx.buffer_layout(Encode::size(msg));
        let buffer = self.stream.alloc(layout.buffer_len()).await?;
        let mut encoder = BEncoder::from(&mut buffer[layout.payload_range()]);
        Encode::encode(msg, &mut encoder);
        Ok(self.encryption_ctx.encrypt_packet(pc, layout, buffer))
    }

    async fn receive_raw<'a, M: Decode<'a>>(&'a mut self) -> TransportResult<M> {
        let pc = self.packets_received;
        self.packets_received += 1;
        self.stream.fetch(2 * PacketLayout::PACKET_LEN_SIZE).await?; // Don't decode size before 8 bytes have arrived

        let len: &[u8] = self.stream.peek_exact(4).await?;
        let buffer_size = self
            .decryption_ctx
            .decrypt_buffer_size(pc, len)
            .filter(|size| *size <= Self::MAX_BUFFER_SIZE)
            .ok_or(TransportError::MessageIntegrity)?;

        let buffer = self.stream.read_exact(buffer_size).await?;
        let packet = self
            .decryption_ctx
            .decrypt_packet(pc, buffer)
            .ok_or(TransportError::MessageIntegrity)?;

        log::error!("PACKET {:?}", packet);

        Decode::decode(&mut BDecoder(&packet[1..])).ok_or(TransportError::DecoderError)
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
