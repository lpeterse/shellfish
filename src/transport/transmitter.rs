use super::*;

pub struct Transmitter<T> {
    pub sender: BufferedSender<WriteHalf<T>>,
    pub receiver: BufferedReceiver<ReadHalf<T>>,
    pub local_id: Identification,
    pub remote_id: Identification,
    pub bytes_sent: u64,
    pub packets_sent: u64,
    pub bytes_received: u64,
    pub packets_received: u64,
    pub encryption_ctx: EncryptionContext,
    pub decryption_ctx: EncryptionContext,
    pub inbox: Option<usize>,
}

impl<T: TransportStream> Transmitter<T> {
    pub async fn new(stream: T, local_id: Identification) -> Result<Self, TransportError> {
        let (rh, wh) = stream.split();
        let mut sender = BufferedSender::new(wh);
        let mut receiver = BufferedReceiver::new(rh);

        Self::send_id(&mut sender, &local_id).await?;
        let remote_id = Self::receive_id(&mut receiver).await?;
        Ok(Self {
            sender,
            receiver,
            local_id,
            remote_id,
            bytes_sent: 0,
            packets_sent: 0,
            bytes_received: 0,
            packets_received: 0,
            encryption_ctx: EncryptionContext::new(),
            decryption_ctx: EncryptionContext::new(),
            inbox: None,
        })
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
        Pin::new(&mut (self.sender))
            .poll_flush(cx)
            .map(|x| x.map_err(Into::into))
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

    pub fn poll_receive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        if self.inbox.is_some() {
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
}
