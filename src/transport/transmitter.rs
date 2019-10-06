use super::*;

pub struct Transmitter<T> {
    sender: BufferedSender<WriteHalf<T>>,
    receiver: BufferedReceiver<ReadHalf<T>>,
    receiver_state: ReceiverState,
    local_id: Identification,
    remote_id: Identification,
    bytes_sent: u64,
    packets_sent: u64,
    bytes_received: u64,
    packets_received: u64,
    encryption_ctx: EncryptionContext,
    decryption_ctx: EncryptionContext,
    alive_timer: Delay,
    alive_interval: std::time::Duration,
    inactivity_timer: Delay,
    inactivity_timeout: std::time::Duration,
}

impl<S: Socket> Transmitter<S> {
    pub async fn new(config: &TransportConfig, socket: S) -> Result<Self, TransportError> {
        let (rh, wh) = socket.split();
        let mut sender = BufferedSender::new(wh);
        let mut receiver = BufferedReceiver::new(rh);

        Self::send_id(&mut sender, &config.identification).await?;
        let remote_id = Self::receive_id(&mut receiver).await?;

        Ok(Self {
            sender,
            receiver,
            receiver_state: ReceiverState::new(),
            local_id: config.identification.clone(),
            remote_id,
            bytes_sent: 0,
            packets_sent: 0,
            bytes_received: 0,
            packets_received: 0,
            encryption_ctx: EncryptionContext::new(),
            decryption_ctx: EncryptionContext::new(),
            alive_timer: Delay::new(config.alive_interval),
            alive_interval: config.alive_interval,
            inactivity_timer: Delay::new(config.inactivity_timeout),
            inactivity_timeout: config.inactivity_timeout,
        })
    }

    pub fn local_id(&self) -> &Identification  {
        &self.local_id
    }

    pub fn remote_id(&self) -> &Identification {
        &self.remote_id
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    pub fn encryption_ctx(&mut self) -> &mut EncryptionContext {
        &mut self.encryption_ctx
    }

    pub fn decryption_ctx(&mut self) -> &mut EncryptionContext {
        &mut self.decryption_ctx
    }

    /// Flush the transport.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        Ok(self.sender.flush().await?)
    }

    /// Check whether the transport is flushed.
    pub fn flushed(&self) -> bool {
        self.sender.flushed()
    }

    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        ready!(self.sender.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    pub fn poll_send<Msg: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let layout = self.encryption_ctx.buffer_layout(Encode::size(msg));
        let buffer = ready!(self.sender.poll_reserve(cx, layout.buffer_len()))?;
        let mut encoder = BEncoder::from(&mut buffer[layout.payload_range()]);
        Encode::encode(msg, &mut encoder);
        let pc = self.packets_sent;
        self.packets_sent += 1;
        self.encryption_ctx.encrypt_packet(pc, layout, buffer);
        self.reset_alive_timer(cx);
        Poll::Ready(Ok(()))
    }

    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let s = self;
        let mut r = Pin::new(&mut s.receiver);
        // Case 1: The packet len has not yet been decrypted
        if s.receiver_state.buffer_len == 0 {
            // Receive at least 8 bytes instead of the required 4
            // in order to impede traffic analysis (as recommended by RFC).
            ready!(r.as_mut().poll_fetch(cx, 2 * PacketLayout::PACKET_LEN_SIZE))?;
            // Decrypt the buffer len. Leave the original packet len field encrypted
            // as it is required for in encrypted form for message intergrity check.
            let mut len = [0; 4];
            len.copy_from_slice(&r.window()[..PacketLayout::PACKET_LEN_SIZE]);
            s.receiver_state.buffer_len = s
                .decryption_ctx.decrypt_len(s.packets_received, len)
                .ok_or(TransportError::BadPacketLength)?;
        }
        // Case 2: The packet len but not the packet has been decrypted
        if s.receiver_state.payload_len == 0 {
            // Wait for the whole packet to arrive (including MAC etc)
            ready!(r.as_mut().poll_fetch(cx, s.receiver_state.buffer_len))?;
            // Try to decrypt the packet.
            let packet = &mut r.window_mut()[..s.receiver_state.buffer_len];
            s.receiver_state.payload_len = s
                .decryption_ctx
                .decrypt_packet(s.packets_received, packet)
                .ok_or(TransportError::MessageIntegrity)?;
        }
        // Case 3: The packet is complete and decrypted in buffer.
        s.reset_inactivity_timer(cx);
        return Poll::Ready(Ok(()))
    }

    pub fn decode<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        let payload_len = self.receiver_state.payload_len;
        assert!(payload_len != 0);
        let payload = &self.receiver.window()[PacketLayout::PAYLOAD_OFFSET..][..payload_len];
        DecodeRef::decode(&mut BDecoder(payload))
    }

    pub fn consume(&mut self) {
        let buffer_len = self.receiver_state.buffer_len;
        assert!(buffer_len != 0);
        self.packets_received += 1;
        self.receiver.consume(buffer_len);
        self.receiver_state.reset();
    }

    /// Send the local identification string.
    async fn send_id(
        socket: &mut BufferedSender<WriteHalf<S>>,
        id: &Identification,
    ) -> Result<(), TransportError> {
        let mut enc = BEncoder::from(socket.reserve(Encode::size(&id) + 2).await?);
        Encode::encode(&id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        socket.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    async fn receive_id(
        socket: &mut BufferedReceiver<ReadHalf<S>>,
    ) -> Result<Identification, TransportError> {
        // Drop lines until remote SSH-2.0- version string is recognized
        loop {
            let line = socket.read_line(Identification::MAX_LEN).await?;
            match DecodeRef::decode(&mut BDecoder(line)) {
                None => (),
                Some(id) => return Ok(id),
            }
        }
    }

    fn reset_alive_timer(&mut self, cx: &mut Context) {
        self.alive_timer.reset(self.alive_interval);
        match self.alive_timer.poll_unpin(cx) {
            Poll::Pending => (),
            _ => panic!("alive_timer fired immediately")
        }
    }

    fn reset_inactivity_timer(&mut self, cx: &mut Context) {
        self.inactivity_timer.reset(self.inactivity_timeout);
        match self.inactivity_timer.poll_unpin(cx) {
            Poll::Pending => (),
            _ => panic!("inactivity_timer fired immediately")
        }
    }

    pub fn check_keep_alive_required(&mut self, cx: &mut Context) -> Result<bool, TransportError> {
        match self.alive_timer.poll_unpin(cx) {
            Poll::Pending => Ok(false),
            Poll::Ready(Ok(())) => Ok(true),
            Poll::Ready(Err(e)) => Err(e.into()),
        }
    }

    pub fn check_inactivity_timeout(&mut self, cx: &mut Context) -> Result<(), TransportError> {
        match self.inactivity_timer.poll_unpin(cx) {
            Poll::Pending => Ok(()),
            Poll::Ready(Ok(())) => Err(TransportError::InactivityTimeout),
            Poll::Ready(Err(e)) => Err(e.into()),
        }
    }
}

struct ReceiverState {
    pub buffer_len: usize,
    pub payload_len: usize,
}

impl ReceiverState {
    pub fn new() -> Self {
        Self {
            buffer_len: 0,
            payload_len: 0,
        }
    }
    pub fn reset(&mut self) {
        self.buffer_len = 0;
        self.payload_len = 0;
    }
}
