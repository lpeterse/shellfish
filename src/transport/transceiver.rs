use super::CipherContext;
use super::CrLf;
use super::Identification;
use super::TransportConfig;
use super::TransportError;
use crate::ready;
use crate::util::buffer::Buffer;
use crate::util::check;
use crate::util::codec::*;
use crate::util::socket::Socket;
use crate::util::poll_fn;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

/// Handles the low-level part of the wire-protocol including framing and cipher.
#[derive(Debug)]
pub struct Transceiver<S: Socket> {
    config: Arc<TransportConfig>,
    /// Underlying socket object for IO
    socket: S,

    /// Total number of bytes received
    rx_bytes: u64,
    /// Total number of packets received
    rx_packets: u64,
    /// Cipher context for decryption
    rx_cipher: CipherContext,
    /// Receive buffer
    rx_buffer: Buffer,
    /// Length of the current packet (including MAC and length field; 0 if no curent packet)
    rx_paclen: usize,
    /// Length of the current message (packet payload without padding)
    rx_msglen: usize,

    /// Total number of bytes sent
    tx_bytes: u64,
    /// Total number of packets sent
    tx_packets: u64,
    /// Cipher context for encryption
    tx_cipher: CipherContext,
    /// Send buffer
    tx_buffer: Buffer,
    /// Length of the current packet (including MAC and length field; 0 if no curent packet)
    tx_paclen: usize,
}

impl<S: Socket> Transceiver<S> {
    /// Create a new transceiver.
    pub fn new(config: &Arc<TransportConfig>, socket: S) -> Self {
        Self {
            config: config.clone(),
            socket,

            rx_bytes: 0,
            rx_packets: 0,
            rx_buffer: Buffer::new(config.rx_buffer_size_min),
            rx_cipher: CipherContext::new(),
            rx_paclen: 0,
            rx_msglen: 0,

            tx_bytes: 0,
            tx_packets: 0,
            tx_buffer: Buffer::new(config.tx_buffer_size_min),
            tx_cipher: CipherContext::new(),
            tx_paclen: 0,
        }
    }

    /// Get the number of bytes received (and consumed).
    pub fn rx_bytes(&self) -> u64 {
        self.rx_bytes
    }

    /// Get the number of bytes sent (eventually not yet flushed).
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
    /// You need to call [Self::consume] afterwards to remove the message from the rx buffer.
    pub fn rx_peek(&mut self, cx: &mut Context) -> Poll<Result<&[u8], TransportError>> {
        let e = || TransportError::InvalidPacket;
        // Case 1: The packet len has not yet been decrypted
        if self.rx_paclen == 0 {
            // Receive at least 8 bytes instead of the required 4
            // in order to impede traffic analysis (as recommended by RFC).
            ready!(self.rx_fill_at_least(cx, 8))?;
            // Decrypt the packet len. Leave the original packet len field encrypted
            // as it is required in encrypted form for message intergrity check.
            let len: &[u8] = self.rx_buffer.as_ref().get(..4).ok_or_else(e)?;
            let len: [u8; 4] = <[u8; 4]>::try_from(len).ok().ok_or_else(e)?;
            let len: usize = self.rx_cipher.decrypt_len(self.rx_packets, len)?;
            let len = 4 + len + self.rx_cipher.mac_len();
            // The total packet length must not exceed a configurable maximum.
            // The default is the defined 35_000 bytes mentioned in the RFC.
            check(len <= self.rx_buffer_size_max()).ok_or(TransportError::InvalidPacketLength)?;
            self.rx_paclen = len;
        }
        // Case 2: The packet len but not the packet has been decrypted
        if self.rx_msglen == 0 {
            // Wait for the whole packet to arrive (including MAC etc)
            ready!(self.rx_fill_at_least(cx, self.rx_paclen))?;
            // Try to decrypt the packet.
            let buf = &mut self.rx_buffer.as_mut()[..self.rx_paclen];
            self.rx_cipher.decrypt(self.rx_packets, buf)?;
            // Try to determine the msg len (by interpreting the padding len)
            self.rx_msglen = self.rx_paclen - self.rx_cipher.mac_len();
            let padding = *buf.get(4).ok_or_else(e)? as usize;
            check(self.rx_msglen > 5 + padding).ok_or_else(e)?;
            self.rx_msglen -= padding;
        }
        // Case 3: The packet is (already) decrypted and available
        let msg = &self.rx_buffer.as_ref()[5..self.rx_msglen];
        Poll::Ready(Ok(msg))
    }

    /// Consume and remove a message from the rx buffer.
    ///
    /// Must be called exactly once for each processed inbound message.
    pub fn rx_consume(&mut self) -> Result<(), TransportError> {
        self.rx_packets += 1;
        self.rx_bytes += self.rx_paclen as u64;
        self.rx_buffer.consume(self.rx_paclen);
        self.rx_paclen = 0;
        self.rx_msglen = 0;
        Ok(())
    }

    /// Poll for buffer space for sending.
    ///
    /// You must call [Self::commit] after you have written the message data to the buffer returned
    /// by this function. You must not call this function again before having called [Self::commit].
    pub fn tx_alloc(
        &mut self,
        cx: &mut Context,
        msglen: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        assert!(self.tx_paclen == 0);
        assert!(msglen <= self.tx_buffer_size_max());

        let maclen = self.tx_cipher().mac_len();
        let padlen = self.tx_cipher().padding_len(msglen);
        let paclen = 1 + msglen + padlen;
        let buflen = 4 + paclen + maclen;

        if buflen > self.tx_buffer.available() {
            // Resize the buffer if it is not already at maximum size.
            // This is done only several times until either the buffer has been resized to its
            // maximum capacity (logarithmic many steps!) or the buffer capacity reached a size
            // that is optimal wrt to the data flow.
            if self.tx_buffer.capacity() < self.tx_buffer_size_max() {
                let old_capacity = self.tx_buffer.capacity();
                let max_capacity = self.tx_buffer_size_max();
                let new_capacity = buflen;
                let new_capacity = std::cmp::max(new_capacity, old_capacity * 2);
                let new_capacity = std::cmp::min(new_capacity, max_capacity);
                self.tx_buffer.increase_capacity(new_capacity);
            }
            // Flush the buffer and return with pending unless it has been flushed completely.
            ready!(self.tx_flush(cx))?;
        }

        // Extend the buffer by required length and create subslice
        let offset = self.tx_buffer.len();
        self.tx_buffer.extend(buflen);
        let buffer = self.tx_buffer.as_mut()[offset..].as_mut();
        // Remember the buffer as allocated by assigning `tx_paclen`
        self.tx_paclen = buflen;
        // Write packet length and number of padding bytes
        let e = || TransportError::InvalidEncoding;
        let mut enc = RefEncoder::new(buffer);
        enc.push_usize(paclen).ok_or_else(e)?;
        enc.push_u8(padlen as u8).ok_or_else(e)?;
        let buffer = buffer[4 + 1..].as_mut();
        // Write zero bytes into the padding area
        for i in buffer[msglen..][..padlen].as_mut() {
            *i = 0x00
        }
        // Return a slice pointing to the message area
        Poll::Ready(Ok(buffer[..msglen].as_mut()))
    }

    /// Commit a message written with [Self::tx_alloc].
    ///
    /// You must call this function exactly once for each invocation of [Self::tx_alloc].
    /// You must call it immediately after you have written your data.
    pub fn tx_commit(&mut self) -> Result<(), TransportError> {
        assert!(self.tx_paclen != 0);
        // Skip over older pending messages that are already encrypted
        let offset = self.tx_buffer.len() - self.tx_paclen;
        let buffer = self.tx_buffer.as_mut()[offset..].as_mut();
        // Encrypt message and adapt various counters
        self.tx_cipher.encrypt(self.tx_packets, buffer)?;
        self.tx_bytes += self.tx_paclen as u64;
        self.tx_packets += 1;
        self.tx_paclen = 0;
        Ok(())
    }

    /// Flush any unsent data.
    ///
    /// The tx buffer is empty when this function returns `Ready(Ok(()))`.
    /// Parts of data may have been transmitted when this functions returns `Pending`.
    pub fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        while self.tx_buffer.len() > 0 {
            let buf = self.tx_buffer.as_ref();
            let written = ready!(AsyncWrite::poll_write(Pin::new(&mut self.socket), cx, &buf))?;
            self.tx_buffer.consume(written);
        }
        Poll::Ready(Ok(()))
    }

    /// Send the local identification string.
    pub async fn tx_id(&mut self, id: &Identification<&'static str>) -> Result<(), TransportError> {
        let data = SshCodec::encode(&CrLf(id)).ok_or(TransportError::InvalidEncoding)?;
        self.socket.write_all(data.as_ref()).await?;
        self.socket.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    ///
    /// The RFC mandates that clients accept other lines before the SSH-* connection string.
    /// This is controlled by the boolean parameter (server does not accept this).
    /// Being defensive, the number of "other lines" is hard-limited (currently 10).
    pub async fn rx_id(&mut self, is_client: bool) -> Result<Identification, TransportError> {
        const MAX_LINE_COUNT: usize = 10;
        const ERR: TransportError = TransportError::InvalidIdentification;

        let mut len = 2;
        let mut line_count = 0;

        while line_count < MAX_LINE_COUNT {
            let buf = self.rx_buffer.as_ref();
            // The first match yields `None` as buffer is empty!
            match buf.get(len - 1) {
                // If slice has line-terminator and starts with SSH-* it must be a version string
                Some(b'\n') if buf.starts_with(Identification::<String>::PREFIX) => {
                    let id: CrLf<_> = SshCodec::decode(&buf[..len]).ok_or(ERR)?;
                    self.rx_buffer.consume(len);
                    return Ok(id.0);
                }
                // If line does not start with SSH-* but this is a client it must be an "other line"
                Some(b'\n') if is_client => {
                    self.rx_buffer.consume(len);
                    len = 2;
                    line_count += 1;
                }
                // If line does not start with SSH-* and this is a server, this is an error
                Some(b'\n') => break,
                // If this is not a line-terminator increment slice index
                Some(_) => len += 1,
                // If the slice ends without line-terminator, read more input (up to 255 bytes)
                None if buf.len() < 255 => {
                    let request_len = std::cmp::max(2, buf.len() + 1);
                    poll_fn(|cx| self.rx_fill_at_least(cx, request_len)).await?
                }
                // It's an error if line is obviously longer than 255 bytes
                None => break,
            }
        }

        Err(ERR)
    }

    /// Fill the rx buffer to contain at least `len` bytes.
    ///
    /// Returns `Pending` unless the buffer contains `len` bytes.
    fn rx_fill_at_least(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<(), TransportError>> {
        let buffer_size_max = self.rx_buffer_size_max();
        assert!(len <= buffer_size_max);

        // Increase the capacity if it is too small for the requested data.
        // The new capacity is one byte more than requested in order to detect when the buffer
        // is smaller than it should be (see below).
        if self.rx_buffer.capacity() < len {
            let new_capacity = len + 1;
            let new_capacity = std::cmp::min(new_capacity, buffer_size_max);
            self.rx_buffer.increase_capacity(new_capacity);
        }

        // Pushback the buffer if it has sufficient capacity but not at the right position.
        // In this case a partial message resides at the right end of the buffer, but the complete
        // message would overlap the buffer boundary. The data to pushback is always relatively
        // small compared to the buffer capacity as it can only be a prefix of a single message.
        if self.rx_buffer.len() + self.rx_buffer.available() < len {
            self.rx_buffer.pushback();
        }

        // Poll the socket unless the buffer contains the desired amount of data.
        // It is tried to read as many bytes as the buffer has available capacity (more than
        // requested). This shall minimize the number of syscalls.
        // Returns with error on unexpected eof or with `Pending` if socket has no more data.
        while self.rx_buffer.len() < len {
            let buf = self.rx_buffer.available_mut();
            let mut buf_ = tokio::io::ReadBuf::new(buf);
            let sock = Pin::new(&mut self.socket);
            ready!(AsyncRead::poll_read(sock, cx, &mut buf_))?;
            let read = buf_.filled().len();
            if read > 0 {
                // Extend the window of meaningful data by number of bytes read.
                self.rx_buffer.extend(read);
            } else {
                // Throw unexpected eof when no bytes have been read.
                let e = Error::new(ErrorKind::UnexpectedEof, "").into();
                return Poll::Ready(Err(e));
            }
        }

        // If the buffer is full this suggests that even more data is available in the socket's
        // receive buffer in kernel space. It is not strictly necessary, but in this case we just
        // double the buffer space (up to a limit) for the next invocation of this function in order
        // to read more data with each single syscall.
        // This is a very conservative way of doing it, but it will keep buffers just as small as
        // necessary unless an application sends vast amounts of data. In this case the buffer will
        // adapt several times (logarithmic!) before reaching its optimal size.
        let capacity = self.rx_buffer.capacity();
        if capacity < buffer_size_max && capacity == self.rx_buffer.len() {
            let new_capacity = 2 * self.rx_buffer.capacity();
            let new_capacity = std::cmp::min(new_capacity, buffer_size_max);
            self.rx_buffer.increase_capacity(new_capacity);
        }

        Poll::Ready(Ok(()))
    }

    /// The maximum size of the receive buffer. The RFC mandates that all implementations must be
    /// able to process packets of at least 35_000 bytes. The config may exceed this value.
    fn rx_buffer_size_max(&self) -> usize {
        std::cmp::max(35_000, self.config.rx_buffer_size_max)
    }

    /// The maximum size of the send buffer. The RFC mandates that all implementations must be
    /// able to process packets of at least 35_000 bytes. The config may exceed this value.
    fn tx_buffer_size_max(&self) -> usize {
        std::cmp::max(35_000, self.config.tx_buffer_size_max)
    }
}
