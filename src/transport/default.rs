use super::*;

/// Implements the transport layer as described in RFC 4253.
///
/// This structure is polymorphic in the socket type (most likely `TcpStream` but other types are
/// used for testing).
#[derive(Debug)]
pub struct DefaultTransport<S: Socket = TcpStream> {
    trx: Transceiver<S>,
    kex: Box<dyn Kex>,
}

impl<S: Socket> DefaultTransport<S> {
    /// Create a new transport acting as client.
    ///
    /// The initial key exchange has been completed successfully when function returns.
    pub async fn connect(
        config: &Arc<TransportConfig>,
        known_hosts: &Arc<dyn KnownHostsLike>,
        hostname: String,
        socket: S,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(socket);
        trx.tx_id(&config.identification).await?;
        let id = trx.rx_id().await?;
        let kex = ClientKex::new(&config, &known_hosts, id, hostname);
        let kex = Box::new(kex);
        let mut transport = Self { trx, kex };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Create a new transport acting as server.
    ///
    /// The initial key exchange has been completed successfully when function returns.
    /// FIXME
    pub async fn accept(
        config: Arc<TransportConfig>,
        _auth_agent: Arc<dyn Agent>,
        socket: S,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(socket);
        trx.tx_id(&config.identification).await?;
        let _id = trx.rx_id().await?;
        let kex = ServerKex::new(&config);
        let kex = Box::new(kex);
        let mut transport = Self { trx, kex };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Initiate a rekeying and wait for it to complete.
    async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex.init(self.trx.tx_bytes(), self.trx.rx_bytes());
        poll_fn(|cx| {
            while self.kex.is_active() {
                // FIXME
                match self.poll_peek(cx).map(|x| x.map(|_| ())) {
                    Poll::Ready(Err(e)) => Err(e)?,
                    Poll::Pending if self.kex.is_active() => return Poll::Pending,
                    _ => return Poll::Ready(Ok(())),
                }
            }
            Poll::Ready(Ok(()))
        })
        .await
    }

    ///
    ///
    /// This function returns `Ready` unless sending a pending kex message blocks.
    fn send_pending_kex_messages(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let sent = self.trx.tx_bytes();
        let rcvd = self.trx.rx_bytes();

        if let Poll::Ready(x) = self.kex.poll_init(cx, sent, rcvd) {
            ready!(self.poll_send_raw(cx, &x?))?;
            log::debug!("Sent MSG_KEX_INIT");
            self.kex.push_init_tx()?;
            ready!(self.poll_flush(cx))?;
        }

        if let Poll::Ready(x) = self.kex.poll_ecdh_init(cx) {
            ready!(self.poll_send_raw(cx, &x?))?;
            log::debug!("Sent MSG_ECDH_INIT");
            self.kex.push_ecdh_init_tx()?;
            ready!(self.poll_flush(cx))?;
        }

        if let Poll::Ready(x) = self.kex.poll_ecdh_reply(cx) {
            ready!(self.poll_send_raw(cx, &x?))?;
            log::debug!("Sent MSG_ECDH_REPLY");
            self.kex.push_ecdh_reply_tx()?;
            ready!(self.poll_flush(cx))?;
        }

        if let Poll::Ready(x) = self.kex.poll_new_keys_tx(cx) {
            let enc = x?;
            ready!(self.poll_send_raw(cx, &MsgNewKeys {}))?;
            log::debug!("Sent MSG_NEWKEYS");
            self.trx
                .tx_cipher()
                .update(enc.clone()) // FIXME clone
                .ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
            self.kex.push_new_keys_tx()?;
            ready!(self.poll_flush(cx))?;
        }

        Poll::Ready(Ok(()))
    }

    // Wenn die Funktion () returned, liegt eine Nicht-Transport Nachricht vor
    fn process_transport_messages(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        loop {
            ready!(self.send_pending_kex_messages(cx))?;
            let buf = ready!(self.trx.rx_peek(cx))?;
            match *buf.get(0).ok_or(TransportError::InvalidPacket)? {
                MsgDisconnect::NUMBER => {
                    // Try to interpret as MSG_DISCONNECT. If successful, convert it into an error
                    // and let the callee handle the termination.
                    let msg: MsgDisconnect =
                        SliceDecoder::decode(buf).ok_or(TransportError::DecoderError)?;
                    log::debug!("Received MSG_DISCONNECT: {:?}", msg.reason);
                    return Poll::Ready(Err(TransportError::DisconnectByPeer(msg.reason)));
                }
                MsgUnimplemented::NUMBER => {
                    // Throw the corresponding error.
                    log::debug!("Received MSG_UNIMPLEMENTED");
                    return Poll::Ready(Err(TransportError::MessageUnimplemented));
                }
                MsgIgnore::NUMBER => {
                    // Try to interpret as MSG_IGNORE. If successful, the message is (as the name
                    // suggests) just ignored. Ignore messages may be introduced any time to impede
                    // traffic analysis and for keep alive.
                    log::debug!("Received MSG_IGNORE");
                    drop(buf);
                    self.trx.rx_consume();
                    continue;
                }
                MsgDebug::NUMBER => {
                    // Try to interpret as MSG_DEBUG. If successful, log as debug and continue.
                    let msg: MsgDebug =
                        SliceDecoder::decode(buf).ok_or(TransportError::DecoderError)?;
                    log::debug!("Received MSG_DEBUG: {:?}", msg.message);
                    self.trx.rx_consume();
                    continue;
                }
                MsgKexInit::<String>::NUMBER => {
                    // Try to interpret as MSG_KEX_INIT. If successful, pass it to the kex handler.
                    // Unless the protocol is violated, kex is in progress afterwards (if not already).
                    log::debug!("Received MSG_KEX_INIT");
                    let msg: MsgKexInit =
                        SliceDecoder::decode(buf).ok_or(TransportError::DecoderError)?;
                    let tx = self.trx.tx_bytes();
                    let rx = self.trx.rx_bytes();
                    self.kex.push_init_rx(tx, rx, msg)?;
                    self.trx.rx_consume();
                    continue;
                }
                MsgKexEcdhReply::<X25519>::NUMBER => {
                    log::debug!("Received MSG_ECDH_REPLY");
                    let msg: MsgKexEcdhReply<X25519> =
                        SliceDecoder::decode(buf).ok_or(TransportError::DecoderError)?;
                    self.kex.push_ecdh_reply_rx(msg)?;
                    self.trx.rx_consume();
                    continue;
                }
                MsgNewKeys::NUMBER => {
                    let dec = ready!(self.kex.poll_new_keys_rx(cx))?;
                    let r = self.trx.rx_cipher().update(dec);
                    r.ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                    self.kex.push_new_keys_rx()?;
                    self.trx.rx_consume();
                    log::debug!("Received MSG_NEWKEYS");
                    continue;
                }
                _ if self.kex.is_receiving_critical() => {
                    return Poll::Ready(Err(TransportError::MessageUnexpected));
                }
                _ => return Poll::Ready(Ok(())),
            }
        }
    }

    /// Poll sending a message.
    ///
    /// Returns `Pending` if the sender does not have enough space and needs to be flushed first.
    /// Resets the alive timer on success.
    pub fn poll_send_raw<Msg: Encode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let buf = ready!(self.trx.tx_alloc(cx, msg.size()))?;
        let mut e = SliceEncoder::new(buf);
        e.push_encode(msg).ok_or(TransportError::EncoderError)?;
        self.commit();
        Poll::Ready(Ok(()))
    }
}

impl<S: Socket> Transport for DefaultTransport<S> {
    fn poll_peek(&mut self, cx: &mut Context) -> Poll<Result<&[u8], TransportError>> {
        // Transport messages are handled internally by this function. In such a case the loop
        // will iterate more than once but always terminate with either Ready or Pending.
        // In case a running kex forbids receiving non-kex packets we need to drive kex to
        // completion first: This means dispatching transport messages only; all other packets
        // will cause an error.
        ready!(self.process_transport_messages(cx))?;
        self.trx.rx_peek(cx)
    }

    fn consume(&mut self) {
        self.trx.rx_consume()
    }

    fn poll_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        // In case a running kex forbids sending no-kex packets we need to drive
        // kex to completion first. This requires dispatching transport messages.
        ready!(self.send_pending_kex_messages(cx))?;
        // Wenn sending critical ist, wurde noch kein MSG_NEW_KEYS gesendet
        while self.kex.is_sending_critical() {
            ready!(self.process_transport_messages(cx))?;
        }
        self.trx.tx_alloc(cx, len)
    }

    fn commit(&mut self) {
        self.trx.tx_commit()
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.trx.tx_flush(cx)
    }

    fn send_disconnect(&mut self, cx: &mut Context, reason: DisconnectReason) {
        let msg = MsgDisconnect::new(reason);
        let _ = self.poll_send_raw(cx, &msg);
        let _ = self.poll_flush(cx);
    }

    fn send_unimplemented(&mut self, cx: &mut Context) {
        let msg = MsgUnimplemented {
            packet_number: self.trx.rx_packets() as u32,
        };
        let _ = self.poll_send_raw(cx, &msg);
        let _ = self.poll_flush(cx);
    }

    fn session_id(&self) -> Result<&SessionId, TransportError> {
        self.kex.session_id()
    }
}
