use super::*;
use crate::util::buffer::Buffer;
use crate::util::check;
use async_std::prelude::*;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};

/// Handles the low-level part of the wire-protocol including framing and cipher.
#[derive(Debug)]
pub struct Transceiver<S: Socket> {
    socket: S,

    /// Total number of bytes received
    rx_bytes: u64,
    /// Total number of packets received
    rx_packets: u64,
    /// Cipher context for decryption
    rx_cipher: CipherContext,
    /// Receive buffer
    rx_buffer: Buffer,
    /// Length of the current packet (0 if no curent packet)
    rx_buflen: usize,
    /// Maximum size of the receive buffer (35_000 usually)
    rx_buflen_max: usize,
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
    /// Length of the current packet (0 if not allocated)
    tx_buflen: usize,
    /// Maximum size of the sent buffer (35_000 usually)
    tx_buflen_max: usize,
}

impl<S: Socket> Transceiver<S> {
    /// Create a new transceiver.
    pub fn new(config: &Arc<TransportConfig>, socket: S) -> Self {
        Self {
            socket,

            rx_bytes: 0,
            rx_packets: 0,
            rx_buffer: Buffer::new(config.rx_buffer_size_min),
            rx_cipher: CipherContext::new(),
            rx_buflen: 0,
            rx_buflen_max: std::cmp::max(PACKET_MAX_LEN, config.rx_buffer_size_max),
            rx_msglen: 0,

            tx_bytes: 0,
            tx_packets: 0,
            tx_buffer: Buffer::new(config.tx_buffer_size_min),
            tx_cipher: CipherContext::new(),
            tx_buflen: 0,
            tx_buflen_max: std::cmp::max(PACKET_MAX_LEN, config.tx_buffer_size_max),
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
        if self.rx_buflen == 0 {
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
            check(len <= self.rx_buflen_max).ok_or(TransportError::InvalidPacketLength)?;
            self.rx_buflen = len;
        }
        // Case 2: The packet len but not the packet has been decrypted
        if self.rx_msglen == 0 {
            // Wait for the whole packet to arrive (including MAC etc)
            ready!(self.rx_fill_at_least(cx, self.rx_buflen))?;
            // Try to decrypt the packet.
            let buf = &mut self.rx_buffer.as_mut()[..self.rx_buflen];
            self.rx_cipher.decrypt(self.rx_packets, buf)?;
            // Try to determine the msg len (by interpreting the padding len)
            self.rx_msglen = self.rx_buflen - self.rx_cipher.mac_len();
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
        self.rx_bytes += self.rx_buflen as u64;
        self.rx_buffer.consume(self.rx_buflen);
        self.rx_buflen = 0;
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
        assert!(self.tx_buflen == 0);
        assert!(msglen <= self.tx_buflen_max);

        let maclen = self.tx_cipher().mac_len();
        let padlen = self.tx_cipher().padding_len(msglen);
        let paclen = 1 + msglen + padlen;
        let buflen = 4 + paclen + maclen;

        if buflen > self.tx_buffer.available() {
            // Resize the buffer if it is not already at maximum size.
            // This is done only several times until either the buffer has been resized to its
            // maximum capacity (logarithmic many steps!) or the buffer capacity reached a size
            // that is optimal wrt to the data flow.
            if self.tx_buffer.capacity() < self.tx_buflen_max {
                let new_capacity = buflen;
                let new_capacity = std::cmp::max(new_capacity, 2 * self.rx_buffer.capacity());
                let new_capacity = std::cmp::min(new_capacity, self.rx_buflen_max);
                self.rx_buffer.increase_capacity(new_capacity);
            }
            // Flush the buffer and return with pending unless it has been flushed completely.
            ready!(self.tx_flush(cx))?;
        }

        // Extend the buffer by required length and create subslice
        let offset = self.tx_buffer.len();
        self.tx_buffer.extend(buflen);
        let buffer = self.tx_buffer.as_mut()[offset..].as_mut();
        // Remember the buffer as allocated by assigning `tx_buflen`
        self.tx_buflen = buflen;
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
        assert!(self.tx_buflen != 0);
        // Skip over older pending messages that are already encrypted
        let offset = self.tx_buffer.len() - self.tx_buflen;
        let buffer = self.tx_buffer.as_mut()[offset..].as_mut();
        // Encrypt message and adapt various counters
        self.tx_cipher.encrypt(self.tx_packets, buffer)?;
        self.tx_bytes += self.tx_buflen as u64;
        self.tx_packets += 1;
        self.tx_buflen = 0;
        Ok(())
    }

    /// Flush any unsent data.
    ///
    /// The tx buffer is empty when this function returns `Ready(Ok(()))`.
    /// Parts of data may have been transmitted when this functions returns `Pending`.
    pub fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        while self.tx_buffer.len() > 0 {
            let bytes = self.tx_buffer.as_ref();
            let written = ready!(Pin::new(&mut self.socket).poll_write(cx, &bytes))?;
            self.tx_buffer.consume(written);
        }
        Poll::Ready(Ok(()))
    }

    /// Send the local identification string.
    pub async fn tx_id(&mut self, id: &Identification<&'static str>) -> Result<(), TransportError> {
        let e = TransportError::InvalidEncoding;
        let data = SshCodec::encode(&CrLf(id)).ok_or(e)?;
        self.socket.write_all(data.as_ref()).await?;
        self.socket.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    pub async fn rx_id(&mut self) -> Result<Identification, TransportError> {
        let e = TransportError::InvalidIdentification;
        let mut len = 2;
        loop {
            let buf = self.rx_buffer.as_ref();
            match self.rx_buffer.as_ref().get(len - 1) {
                Some(b'\n') if buf.starts_with(Identification::<String>::PREFIX) => {
                    let id: CrLf<_> = SshCodec::decode(&buf[..len]).ok_or(e)?;
                    self.rx_buffer.consume(len);
                    return Ok(id.0);
                }
                Some(b'\n') => {
                    self.rx_buffer.consume(len);
                    len = 2
                }
                Some(_) => len += 1,
                None if buf.len() < 255 => {
                    let request_len = buf.len() + 1;
                    poll_fn(|cx| self.rx_fill_at_least(cx, request_len)).await?
                }
                None => return Err(e),
            }
        }
    }

    /// Fill the rx buffer to contain at least `len` bytes.
    ///
    /// Returns `Pending` unless the buffer contains `len` bytes.
    fn rx_fill_at_least(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<(), TransportError>> {
        assert!(len <= self.rx_buflen_max);

        // Increase the capacity if it is too small for the requested data.
        // The new capacity is one byte more than requested in order to detect when the buffer
        // is smaller than it should be (see below).
        if self.rx_buffer.capacity() < len {
            let new_capacity = len + 1;
            let new_capacity = std::cmp::min(new_capacity, self.rx_buflen_max);
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
            let read = ready!(Pin::new(&mut self.socket).poll_read(cx, buf))?;
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
        if capacity < self.rx_buflen_max && capacity == self.rx_buffer.len() {
            let new_capacity = 2 * self.rx_buffer.capacity();
            let new_capacity = std::cmp::min(new_capacity, self.rx_buflen_max);
            self.rx_buffer.increase_capacity(new_capacity);
        }

        Poll::Ready(Ok(()))
    }
}
