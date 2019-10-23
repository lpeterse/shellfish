use super::*;
use crate::util::assume;

use std::ops::Add;

pub struct Transmitter<T> {
    sender: BufferedSender<WriteHalf<T>>,
    receiver: BufferedReceiver<ReadHalf<T>>,
    receiver_state: ReceiverState,
    local_id: Identification<&'static str>,
    remote_id: Identification<String>,
    bytes_sent: u64,
    packets_sent: u64,
    bytes_received: u64,
    packets_received: u64,
    encryption_ctx: CipherContext,
    decryption_ctx: CipherContext,
    alive_timer: Delay,
    alive_interval: std::time::Duration,
    inactivity_timer: Delay,
    inactivity_timeout: std::time::Duration,
}

impl<S: Socket> Transmitter<S> {
    pub async fn new<C: TransportConfig>(config: &C, socket: S) -> Result<Self, TransportError> {
        let (rh, wh) = socket.split();
        let mut sender = BufferedSender::new(wh);
        let mut receiver = BufferedReceiver::new(rh);

        Self::send_id(&mut sender, config.identification()).await?;
        let remote_id = Self::receive_id(&mut receiver).await?; // FIXME TIMEOUT

        Ok(Self {
            sender,
            receiver,
            receiver_state: ReceiverState::new(),
            local_id: config.identification().clone(),
            remote_id,
            bytes_sent: 0,
            packets_sent: 0,
            bytes_received: 0,
            packets_received: 0,
            encryption_ctx: CipherContext::new(),
            decryption_ctx: CipherContext::new(),
            alive_timer: Delay::new(config.alive_interval()),
            alive_interval: config.alive_interval(),
            inactivity_timer: Delay::new(config.inactivity_timeout()),
            inactivity_timeout: config.inactivity_timeout(),
        })
    }

    pub fn local_id(&self) -> &Identification<&'static str> {
        &self.local_id
    }

    pub fn remote_id(&self) -> &Identification<String> {
        &self.remote_id
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    pub fn packets_sent(&self) -> u64 {
        self.packets_sent
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    pub fn packets_received(&self) -> u64 {
        self.packets_received
    }

    pub fn encryption_ctx(&mut self) -> &mut CipherContext {
        &mut self.encryption_ctx
    }

    pub fn decryption_ctx(&mut self) -> &mut CipherContext {
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
        let packet = self.encryption_ctx.packet(msg);
        let buffer: &mut [u8] = ready!(self.sender.poll_reserve(cx, packet.size()))?;
        packet.encode(&mut BEncoder::from(&mut buffer[..]));
        self.encryption_ctx.encrypt(self.packets_sent, buffer);
        self.packets_sent += 1;
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
            ready!(r.as_mut().poll_fetch(cx, 2 * 4))?;
            // Decrypt the buffer len. Leave the original packet len field encrypted
            // as it is required for in encrypted form for message intergrity check.
            let mut len = [0; 4];
            len.copy_from_slice(&r.window()[..4]);
            s.receiver_state.buffer_len = s
                .decryption_ctx
                .decrypt_len(s.packets_received, len)
                .ok_or(TransportError::BadPacketLength)?;
        }
        // Case 2: The packet len but not the packet has been decrypted
        if s.receiver_state.packet_len == 0 {
            // Wait for the whole packet to arrive (including MAC etc)
            ready!(r.as_mut().poll_fetch(cx, s.receiver_state.buffer_len))?;
            // Try to decrypt the packet.
            let packet = &mut r.window_mut()[..s.receiver_state.buffer_len];
            let packet_len = s
                .decryption_ctx
                .decrypt(s.packets_received, packet)
                .ok_or(TransportError::MessageIntegrity)?;

            s.receiver_state.packet_len = packet_len;
        }
        // Case 3: The packet is complete and decrypted in buffer.
        s.reset_inactivity_timer(cx);
        return Poll::Ready(Ok(()));
    }

    pub fn decode<'a, Msg: DecodeRef<'a>>(&'a mut self) -> Option<Msg> {
        let packet_len = self.receiver_state.packet_len;
        assert!(packet_len != 0);
        let packet = &self.receiver.window()[4..][..packet_len];
        let padding: usize = *packet.get(0)? as usize;
        assume(packet_len >= 1 + padding)?;
        let payload = &packet[1..][..packet_len - 1 - padding];
        BDecoder::decode(payload)
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
        id: &Identification<&'static str>,
    ) -> Result<(), TransportError> {
        let mut enc = BEncoder::from(socket.reserve(Encode::size(id) + 2).await?);
        Encode::encode(&id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        socket.flush().await?;
        Ok(())
    }

    /// Receive the remote identification string.
    async fn receive_id(
        socket: &mut BufferedReceiver<ReadHalf<S>>,
    ) -> Result<Identification<String>, TransportError> {
        // Drop lines until remote SSH-2.0- version string is recognized
        loop {
            let line: &[u8] = socket.read_line(Identification::<String>::MAX_LEN).await?;
            match Decode::decode(&mut BDecoder::from(&line)) {
                None => (),
                Some(id) => return Ok(id),
            }
        }
    }

    fn reset_alive_timer(&mut self, cx: &mut Context) {
        let time = std::time::Instant::now().add(self.alive_interval);
        self.alive_timer.reset(time);
        match self.alive_timer.poll_unpin(cx) {
            Poll::Pending => (),
            _ => panic!("alive_timer fired immediately"),
        }
    }

    fn reset_inactivity_timer(&mut self, cx: &mut Context) {
        let time = std::time::Instant::now().add(self.inactivity_timeout);
        self.inactivity_timer.reset(time);
        match self.inactivity_timer.poll_unpin(cx) {
            Poll::Pending => (),
            _ => panic!("inactivity_timer fired immediately"),
        }
    }

    /// Send a keep alive message when determined to be required.
    /// Like above, the call registers the timer for wakeup. The alive timer is reset
    /// automatically when the message has been sent successfully.
    pub fn poll_keepalive(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        match self.alive_timer.poll_unpin(cx) {
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
        match self.inactivity_timer.poll_unpin(cx) {
            Poll::Pending => (),
            Poll::Ready(()) => Err(TransportError::InactivityTimeout)?,
        }
        Poll::Ready(Ok(()))
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
