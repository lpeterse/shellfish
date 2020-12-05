use super::*;
use crate::transport::PACKET_LEN_BYTES;
use crate::util::buffer::Buffer;

use async_std::prelude::*;
use std::io::{Error, ErrorKind};

/// The MSG_KEX_INIT message is roughly about 1000 bytes. All other messages
/// are smaller unless the client starts data transfer with larger chunk size
/// for which the buffer automatically adjusts on demand.
const INITIAL_BUFFER_SIZE: usize = 1100;

/// The `Transceiver` handles the low-level part of the wire-protocol including framing and cipher.
#[derive(Debug)]
pub struct Transceiver<S: Socket> {
    socket: S,

    rx_bytes: u64,
    rx_packets: u64,
    rx_cipher: CipherContext,
    rx_buffer: Buffer,
    rx_length: usize,
    rx_ready: bool,

    tx_bytes: u64,
    tx_packets: u64,
    tx_cipher: CipherContext,
    tx_buffer: Buffer,
    tx_alloc: usize,
}

impl<S: Socket> Transceiver<S> {
    /// Create a new transceiver.
    pub fn new(socket: S) -> Self {
        Self {
            socket,

            rx_bytes: 0,
            rx_packets: 0,
            rx_buffer: Buffer::new(INITIAL_BUFFER_SIZE),
            rx_cipher: CipherContext::new(),
            rx_length: 0,
            rx_ready: false,

            tx_bytes: 0,
            tx_packets: 0,
            tx_buffer: Buffer::new(INITIAL_BUFFER_SIZE),
            tx_cipher: CipherContext::new(),
            tx_alloc: 0,
        }
    }

    pub fn rx_bytes(&self) -> u64 {
        self.rx_bytes
    }

    pub fn rx_packets(&self) -> u64 {
        self.rx_packets
    }

    pub fn tx_bytes(&self) -> u64 {
        self.tx_bytes
    }

    /// Get mutable access to the encryption context used by the transceiver.
    pub fn tx_cipher(&mut self) -> &mut CipherContext {
        &mut self.tx_cipher
    }

    /// Get mutable access to the decryption context used by the transceiver.
    pub fn rx_cipher(&mut self) -> &mut CipherContext {
        &mut self.rx_cipher
    }

    /// Poll receiving a message.
    ///
    /// When `Ready`, `decode` and `consume` shall be used to process the message.
    pub fn rx_peek(&mut self, cx: &mut Context) -> Poll<Result<&[u8], TransportError>> {
        let mac_len = self.rx_cipher.mac_len();
        // Case 1: The packet len has not yet been decrypted
        if self.rx_length == 0 {
            // Receive at least 8 bytes instead of the required 4
            // in order to impede traffic analysis (as recommended by RFC).
            ready!(self.tx_fill_at_least(cx, 2 * PACKET_LEN_BYTES))?;
            // Decrypt the buffer len. Leave the original packet len field encrypted
            // as it is required in encrypted form for message intergrity check.
            let mut len = [0; 4];
            len.copy_from_slice(&self.rx_buffer.as_ref()[..4]);
            let len = self
                .rx_cipher
                .decrypt_len(self.rx_packets, len)
                .ok_or(TransportError::BadPacketLength)?;
            self.rx_length = PACKET_LEN_BYTES + len + mac_len;
        }
        // Case 2: The packet len but not the packet has been decrypted
        if !self.rx_ready {
            // Wait for the whole packet to arrive (including MAC etc)
            ready!(self.tx_fill_at_least(cx, self.rx_length))?;
            // Try to decrypt the packet.
            let packet = &mut self.rx_buffer.as_mut()[..self.rx_length];
            self.rx_cipher.decrypt(self.rx_packets, packet)?;
            self.rx_ready = true;
        }
        // Case 3: The packet is complete and decrypted in buffer.
        const ERR: TransportError = TransportError::InvalidPacket;
        let buf = &self.rx_buffer.as_ref()[..self.rx_length];
        let padding_len = *buf.get(PACKET_LEN_BYTES).ok_or(ERR)? as usize;
        let payload = &buf
            .get(PACKET_LEN_BYTES + PADDING_LEN_BYTES..buf.len() - mac_len - padding_len)
            .ok_or(ERR)?;
        return Poll::Ready(Ok(payload));
    }

    /// Consume a decoded message and remove it from the input buffer.
    ///
    /// Shall be called _after_ `decode`.
    pub fn rx_consume(&mut self) {
        assert!(self.rx_ready && self.rx_length != 0);

        self.rx_packets += 1;
        self.rx_bytes += self.rx_length as u64;
        self.rx_buffer.consume(self.rx_length);
        self.rx_length = 0;
        self.rx_ready = false;
    }

    pub fn tx_alloc(
        &mut self,
        cx: &mut Context,
        payload_len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        assert!(self.tx_alloc == 0);

        let mac_len = self.tx_cipher().mac_len();
        let padding_len = self.tx_cipher().padding_len(payload_len);
        let packet_len = PADDING_LEN_BYTES + payload_len + padding_len;
        let buffer_len = PACKET_LEN_BYTES + packet_len + mac_len;
        // If the total capacity is insufficient, resize the buffer.
        if buffer_len > self.tx_buffer.capacity() {
            ready!(self.tx_flush(cx))?;
            self.tx_buffer.increase_capacity(buffer_len)
        }
        // If the available space is still insufficient, try to flush the buffer.
        if buffer_len > self.tx_buffer.available() {
            ready!(self.tx_flush(cx))?;
        }
        // Extend the buffer by required length and create subslice.
        let offset = self.tx_buffer.len();
        self.tx_buffer.extend(buffer_len);
        self.tx_alloc = buffer_len;
        let buffer = self.tx_buffer.as_mut()[offset..].as_mut();
        // Write packet length and number of padding bytes.
        let error = TransportError::EncoderError;
        let mut enc = SliceEncoder::new(buffer);
        enc.push_u32be(packet_len as u32).ok_or(error)?;
        enc.push_u8(padding_len as u8).ok_or(error)?;
        let buffer = buffer[PACKET_LEN_BYTES + PADDING_LEN_BYTES..].as_mut();
        let buffer = buffer[..payload_len + padding_len].as_mut();
        // Write zero bytes into the padding area.
        for i in buffer[payload_len..].as_mut() {
            *i = 0x00
        }
        // Return a slice pointing to the payload area
        Poll::Ready(Ok(buffer[..payload_len].as_mut()))
    }

    pub fn tx_commit(&mut self) {
        assert!(self.tx_alloc != 0);

        let offset = self.tx_buffer.len() - self.tx_alloc;
        let buffer = self.tx_buffer.as_mut()[offset..].as_mut();

        self.tx_cipher.encrypt(self.tx_packets, buffer).unwrap(); // FIXME
        self.tx_bytes += self.tx_alloc as u64;
        self.tx_packets += 1;
        self.tx_alloc = 0;
    }

    /// Poll the sender to flush any unsent data.
    pub fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        while self.tx_buffer.len() > 0 {
            let written =
                ready!(Pin::new(&mut self.socket).poll_write(cx, &self.tx_buffer.as_ref()))?;
            self.tx_buffer.consume(written);
        }
        Poll::Ready(Ok(()))
    }

    /// Send the local identification string.
    pub async fn tx_id(&mut self, id: &Identification<&'static str>) -> Result<(), TransportError> {
        let data = SliceEncoder::encode(&CrLf(id));
        self.socket.write_all(data.as_ref()).await?;
        self.socket.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    pub async fn rx_id(&mut self) -> Result<Identification, TransportError> {
        const ERR: TransportError = TransportError::InvalidIdentification;

        let mut len = 2;
        loop {
            let buf = self.rx_buffer.as_ref();
            match self.rx_buffer.as_ref().get(len - 1) {
                Some(b'\n') if buf.starts_with(Identification::<String>::PREFIX) => {
                    let id: CrLf<_> = SliceDecoder::decode(&buf[..len]).ok_or(ERR)?;
                    self.rx_buffer.consume(len);
                    return Ok(id.0);
                }
                Some(b'\n') => {
                    self.rx_buffer.consume(len);
                    len = 2
                }
                Some(_) => len += 1,
                None if buf.len() < 255 => poll_fn(|cx| self.tx_fill(cx)).await?,
                None => return Err(ERR),
            }
        }
    }

    fn tx_fill(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        if self.rx_buffer.available() == 0 {
            self.rx_buffer.pushback()
        }
        if self.rx_buffer.available() == 0 {
            self.rx_buffer
                .increase_capacity(2 * self.rx_buffer.capacity());
        }
        // Poll-read the underlying socket. This is always safe as
        // the remaining rx.buffer capacity is recalculated/adapted on every poll.
        let read =
            ready!(Pin::new(&mut self.socket).poll_read(cx, self.rx_buffer.available_mut()))?;
        if read > 0 {
            self.rx_buffer.extend(read);
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "").into()))
        }
    }

    fn tx_fill_at_least(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<(), TransportError>> {
        while self.rx_buffer.len() < len {
            ready!(self.tx_fill(cx))?
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::socket::DummySocket;

    #[async_std::test]
    async fn tx_id_simple() {
        let id = Identification::default();
        let (int, ext) = DummySocket::new();
        let mut ext = ext;
        let mut trx = Transceiver::new(int);
        assert_eq!(trx.tx_id(&id).await, Ok(()));
        drop(trx);
        let mut actual = Vec::new();
        let expected = [
            83, 83, 72, 45, 50, 46, 48, 45, 114, 115, 115, 104, 95, 48, 46, 49, 46, 48, 13, 10,
        ];
        assert_eq!(ext.read_to_end(&mut actual).await.unwrap(), expected.len());
        assert_eq!(actual, expected);
    }

    #[async_std::test]
    async fn rx_id_simple() {
        let (int, ext) = DummySocket::new();
        let mut ext = ext;
        let mut trx = Transceiver::new(int);
        assert_eq!(
            ext.write_all(&[
                83, 83, 72, 45, 50, 46, 48, 45, 114, 115, 115, 104, 95, 48, 46, 49, 46, 48, 13, 10,
            ])
            .await
            .unwrap(),
            ()
        );
        assert_eq!(trx.rx_id().await, Ok(Identification::default().into()));
    }
}
