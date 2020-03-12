use super::*;
use crate::util::assume;

use async_std::future::Future;

/// The `Transceiver` handles the low-level part of the wire-protocol including framing and cipher.
pub struct Transceiver<S: Socket> {
    socket: Buffered<S>,
    receiver_state: ReceiverState,
    bytes_sent: u64,
    packets_sent: u64,
    bytes_received: u64,
    packets_received: u64,
    encryption_ctx: CipherContext,
    decryption_ctx: CipherContext,
    local_inactivity_timer: Delay,
    local_inactivity_timeout: std::time::Duration,
    remote_inactivity_timer: Delay,
    remote_inactivity_timeout: std::time::Duration,
}

impl<S: Socket> Transceiver<S> {
    /// Create a new transceiver.
    ///
    /// This function also performs the identification string exchange which may fail for different
    /// reasons. An error is returned in this case.
    pub fn new<C: TransportConfig>(config: &C, socket: S) -> Self {
        Self {
            socket: Buffered::new(socket),
            receiver_state: ReceiverState::new(),
            bytes_sent: 0,
            packets_sent: 0,
            bytes_received: 0,
            packets_received: 0,
            encryption_ctx: CipherContext::new(),
            decryption_ctx: CipherContext::new(),
            local_inactivity_timer: Delay::new(config.alive_interval()),
            local_inactivity_timeout: config.alive_interval(),
            remote_inactivity_timer: Delay::new(config.inactivity_timeout()),
            remote_inactivity_timeout: config.inactivity_timeout(),
        }
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    pub fn packets_received(&self) -> u64 {
        self.packets_received
    }

    /// Get mutable access to the encryption context used by the transceiver.
    pub fn encryption_ctx(&mut self) -> &mut CipherContext {
        &mut self.encryption_ctx
    }

    /// Get mutable access to the decryption context used by the transceiver.
    pub fn decryption_ctx(&mut self) -> &mut CipherContext {
        &mut self.decryption_ctx
    }

    /// Ask whether the sender contains unsent data.
    pub fn flushed(&self) -> bool {
        self.socket.flushed()
    }

    /// Poll the sender to flush any unsent data.
    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.socket.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    /// Poll sending a message.
    ///
    /// Returns `Pending` if the sender does not have enough space and needs to be flushed first.
    /// Resets the alive timer on success.
    pub fn poll_send<Msg: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let packet = self.encryption_ctx.packet(msg);
        let buffer: &mut [u8] = ready!(self.socket.poll_extend(cx, packet.size()))?;
        packet.encode(&mut BEncoder::from(&mut buffer[..]));
        self.encryption_ctx.encrypt(self.packets_sent, buffer);
        self.packets_sent += 1;
        self.bytes_sent += packet.size() as u64;
        self.reset_local_inactivity_timer(cx)?;
        Poll::Ready(Ok(()))
    }

    /// Poll receiving a message.
    ///
    /// When `Ready`, `decode` and `consume` shall be used to process the message.
    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let s = self;
        let mut r = Pin::new(&mut s.socket);
        // Case 1: The packet len has not yet been decrypted
        if s.receiver_state.buffer_len == 0 {
            // Receive at least 8 bytes instead of the required 4
            // in order to impede traffic analysis (as recommended by RFC).
            ready!(r.as_mut().poll_fill_exact(cx, 2 * 4))?;
            // Decrypt the buffer len. Leave the original packet len field encrypted
            // as it is required in encrypted form for message intergrity check.
            let mut len = [0; 4];
            len.copy_from_slice(&Buffered::as_ref(&r)[..4]);
            s.receiver_state.buffer_len = s
                .decryption_ctx
                .decrypt_len(s.packets_received, len)
                .ok_or(TransportError::BadPacketLength)?;
        }
        // Case 2: The packet len but not the packet has been decrypted
        if s.receiver_state.packet_len == 0 {
            // Wait for the whole packet to arrive (including MAC etc)
            ready!(r.as_mut().poll_fill_exact(cx, s.receiver_state.buffer_len))?;
            // Try to decrypt the packet.
            let packet = &mut Buffered::as_mut(&mut r)[..s.receiver_state.buffer_len];
            let packet_len = s
                .decryption_ctx
                .decrypt(s.packets_received, packet)
                .ok_or(TransportError::MessageIntegrity)?;

            s.receiver_state.packet_len = packet_len;
        }
        // Case 3: The packet is complete and decrypted in buffer.
        s.reset_remote_inactivity_timer(cx)?;
        return Poll::Ready(Ok(()));
    }

    /// Decode a decrypted message.
    ///
    /// Shall be called _after_ `poll_receive` was ready.
    pub fn decode<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        let packet_len = self.receiver_state.packet_len;
        assert!(packet_len != 0);
        let packet = &self.socket.as_ref()[4..][..packet_len];
        let padding: usize = *packet.get(0)? as usize;
        assume(packet_len >= 1 + padding)?;
        let payload = &packet[1..][..packet_len - 1 - padding];
        BDecoder::decode(payload)
    }

    /// Consume a decoded message and remove it from the input buffer.
    ///
    /// Shall be called _after_ `decode`.
    pub fn consume(&mut self) {
        let buffer_len = self.receiver_state.buffer_len;
        assert!(buffer_len != 0);
        self.packets_received += 1;
        self.bytes_received += buffer_len as u64;
        self.socket.consume(buffer_len);
        self.receiver_state.reset();
    }

    /// Send a keep alive message when determined to be required.
    /// Like above, the call registers the timer for wakeup. The alive timer is reset
    /// automatically when the message has been sent successfully.
    pub fn poll_keepalive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        match Future::poll(Pin::new(&mut self.local_inactivity_timer), cx) {
            Poll::Pending => (),
            Poll::Ready(()) => {
                ready!(self.poll_send(cx, &MsgIgnore::new()))?;
                log::debug!("Sent MSG_IGNORE (as keep-alive)");
                ready!(self.poll_flush(cx))?;
            }
        }
        Poll::Ready(Ok(()))
    }

    /// The inactivity check causes an error in case of timeout and falls through else.
    /// Calling it also registers the timer for wakeup (consider this when reordering code).
    pub fn poll_inactivity(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        match Future::poll(Pin::new(&mut self.remote_inactivity_timer), cx) {
            Poll::Pending => (),
            Poll::Ready(()) => Err(TransportError::InactivityTimeout)?,
        }
        Poll::Ready(Ok(()))
    }

    /// Send the local identification string.
    pub async fn send_id(
        &mut self,
        id: &Identification<&'static str>,
    ) -> Result<(), TransportError> {
        let len = Encode::size(id) + 2;
        poll_fn(|cx| {
            let buf = ready!(self.socket.poll_extend(cx, len))?;
            let mut enc = BEncoder::from(buf);
            Encode::encode(id, &mut enc);
            enc.push_u8('\r' as u8);
            enc.push_u8('\n' as u8);
            Poll::Ready(Ok::<(), TransportError>(()))
        })
        .await?;
        self.socket.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    pub async fn receive_id(&mut self) -> Result<Identification, TransportError> {
        // Drop lines until remote SSH-2.0- version string is recognized
        let mut len = 0;
        loop {
            match self.socket.as_ref().get(len + 1) {
                Some(0x0a) if self.socket.as_ref().starts_with(b"SSH-2.0") => break,
                Some(0x0a) => {
                    self.socket.consume(len + 2);
                    len = 0;
                }
                Some(_) => len += 1,
                None if self.socket.as_ref().len() < 255 => self.socket.fill().await?,
                None => Err(TransportError::DecoderError)?,
            }
        }
        let mut d = BDecoder(&self.socket.as_ref()[..len]);
        let id = Decode::decode(&mut d).ok_or(TransportError::DecoderError)?;
        self.socket.consume(len + 2);
        Ok(id)
    }

    /// Resets the local inactivity timer to the configured timespan and registers it for wakeup.
    fn reset_local_inactivity_timer(&mut self, cx: &mut Context) -> Result<(), TransportError> {
        self.local_inactivity_timer
            .reset(self.local_inactivity_timeout);
        match Future::poll(Pin::new(&mut self.local_inactivity_timer), cx) {
            Poll::Pending => Ok(()),
            // Shall not happen, but if it does we rather convert it to an error instead of a panic
            _ => Err(TransportError::InactivityTimeout),
        }
    }

    /// Resets the remote inactivity timer to the configured timespan and registers it for wakeup.
    fn reset_remote_inactivity_timer(&mut self, cx: &mut Context) -> Result<(), TransportError> {
        self.remote_inactivity_timer
            .reset(self.remote_inactivity_timeout);
        match Future::poll(Pin::new(&mut self.remote_inactivity_timer), cx) {
            Poll::Pending => Ok(()),
            // Shall not happen, but if it does we rather convert it to an error instead of a panic
            _ => Err(TransportError::InactivityTimeout),
        }
    }
}

struct ReceiverState {
    pub buffer_len: usize,
    pub packet_len: usize,
}

impl ReceiverState {
    pub fn new() -> Self {
        Self {
            buffer_len: 0,
            packet_len: 0,
        }
    }
    pub fn reset(&mut self) {
        self.buffer_len = 0;
        self.packet_len = 0;
    }
}
