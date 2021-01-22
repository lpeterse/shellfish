use super::transceiver::*;
use crate::util::runtime::{Socket, TcpStream};
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
        socket: S,
        host_name: &str,
        host_port: u16,
        host_verifier: &Arc<dyn HostVerifier>,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(config, socket);
        trx.tx_id(&config.identification).await?;
        let id = trx.rx_id(true).await?;
        let kex = ClientKex::new(config, id, host_name, host_port, host_verifier);
        let kex = Box::new(kex);
        let mut transport = Self { trx, kex };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Create a new transport acting as server.
    ///
    /// The initial key exchange has been completed successfully when function returns.
    pub async fn accept(
        config: &Arc<TransportConfig>,
        _agent: &Arc<dyn AuthAgent>,
        socket: S,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(&config, socket);
        trx.tx_id(&config.identification).await?;
        let id = trx.rx_id(false).await?;
        let kex = ServerKex::new(config, id);
        let kex = Box::new(kex);
        let mut transport = Self { trx, kex };
        transport.rekey().await?;
        Ok(transport)
    }

    /// Initiate a rekeying and wait for it to complete.
    async fn rekey(&mut self) -> Result<(), TransportError> {
        self.kex.init(self.trx.tx_bytes(), self.trx.rx_bytes());
        poll_fn(|cx| self.poll(cx, Condition::KexComplete)).await
    }

    /// Process all internal tasks like kex and all kinds of transport specific messages.
    fn poll(&mut self, cx: &mut Context, cond: Condition) -> Poll<Result<(), TransportError>> {
        let readable = loop {
            let buf = match self.trx.rx_peek(cx) {
                Poll::Ready(Ok(buf)) => buf,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => break false,
            };

            match *buf.get(0).ok_or(TransportError::InvalidPacket)? {
                MsgDisconnect::NUMBER => {
                    // Try to interpret as MSG_DISCONNECT. If successful, convert it into an error
                    // and let the callee handle the termination.
                    let msg: MsgDisconnect =
                        SshCodec::decode(buf).ok_or(TransportError::InvalidEncoding)?;
                    log::debug!("Received MSG_DISCONNECT: {:?}", msg.reason);
                    return Poll::Ready(Err(TransportError::DisconnectByPeer(msg.reason)));
                }
                MsgUnimplemented::NUMBER => {
                    // Try to interpret as MSG_UNIMPLEMENTED. Throw error as long as there is no
                    // need to handle it in a more sophisticated way.
                    let msg: MsgUnimplemented =
                        SshCodec::decode(buf).ok_or(TransportError::InvalidEncoding)?;
                    log::error!("Received MSG_UNIMPLEMENTED: packet {}", msg.packet_number);
                    return Poll::Ready(Err(TransportError::InvalidState));
                }
                MsgIgnore::NUMBER => {
                    // Try to interpret as MSG_IGNORE. If successful, the message is (as the name
                    // suggests) just ignored. Ignore messages may be introduced any time to impede
                    // traffic analysis and for keep alive.
                    log::debug!("Received MSG_IGNORE");
                    self.trx.rx_consume()?;
                }
                MsgDebug::NUMBER => {
                    // Try to interpret as MSG_DEBUG. If successful, log as debug and continue.
                    let msg: MsgDebug =
                        SshCodec::decode(buf).ok_or(TransportError::InvalidEncoding)?;
                    log::debug!("Received MSG_DEBUG: {:?}", msg.message);
                    self.trx.rx_consume()?;
                }
                MsgKexInit::<String>::NUMBER => {
                    // Try to interpret as MSG_KEX_INIT. If successful, pass it to the kex handler.
                    // Unless the protocol is violated, kex is in progress afterwards (if not already).
                    log::debug!("Received MSG_KEX_INIT");
                    let msg: MsgKexInit =
                        SshCodec::decode(buf).ok_or(TransportError::InvalidEncoding)?;
                    let tx = self.trx.tx_bytes();
                    let rx = self.trx.rx_bytes();
                    self.kex.push_init_rx(tx, rx, msg)?;
                    self.trx.rx_consume()?;
                }
                MsgKexEcdhReply::<X25519>::NUMBER => {
                    log::debug!("Received MSG_ECDH_REPLY");
                    let msg: MsgKexEcdhReply<X25519> =
                        SshCodec::decode(buf).ok_or(TransportError::InvalidEncoding)?;
                    self.kex.push_ecdh_reply_rx(msg)?;
                    self.trx.rx_consume()?;
                }
                MsgNewKeys::NUMBER => {
                    let dec = ready!(self.kex.poll_new_keys_rx(cx))?;
                    let r = self.trx.rx_cipher().update(dec);
                    r.ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                    self.kex.push_new_keys_rx()?;
                    self.trx.rx_consume()?;
                    log::debug!("Received MSG_NEWKEYS");
                }
                _ if self.kex.is_receiving_critical() => {
                    return Poll::Ready(Err(TransportError::InvalidState))
                }
                _ => break true,
            }
        };

        if !self.kex.is_active() {
            let txb = self.trx.tx_bytes();
            let rxb = self.trx.rx_bytes();
            self.kex.init_if_necessary(cx, txb, rxb);
        }

        if self.kex.is_active() {
            if let Some(x) = self.kex.peek_init(cx) {
                ready!(self.tx_msg(cx, &x))?;
                log::debug!("Sent MSG_KEX_INIT");
                self.kex.push_init_tx()?;
                ready!(self.trx.tx_flush(cx))?;
            }

            if let Some(x) = self.kex.peek_ecdh_init(cx)? {
                ready!(self.tx_msg(cx, &x))?;
                log::debug!("Sent MSG_ECDH_INIT");
                self.kex.push_ecdh_init_tx()?;
                ready!(self.trx.tx_flush(cx))?;
            }

            if let Some(x) = self.kex.peek_ecdh_reply(cx)? {
                ready!(self.tx_msg(cx, &x))?;
                log::debug!("Sent MSG_ECDH_REPLY");
                self.kex.push_ecdh_reply_tx()?;
                ready!(self.trx.tx_flush(cx))?;
            }

            if let Some(x) = ready!(self.kex.poll_new_keys_tx(cx))? {
                ready!(self.tx_msg(cx, &MsgNewKeys))?;
                log::debug!("Sent MSG_NEWKEYS");
                self.trx
                    .tx_cipher()
                    .update(x)
                    .ok_or(TransportError::NoCommonEncryptionAlgorithm)?;
                self.kex.push_new_keys_tx()?;
                ready!(self.trx.tx_flush(cx))?;
            }
        }

        match cond {
            // If `readable` is false it is guaranteed that the socket is blocked on IO and it is
            // safe to return `Pending` (see above).
            Condition::Readable if readable => Poll::Ready(Ok(())),
            Condition::Readable => Poll::Pending,
            // The transport is writable unless kex is critical for sending. The situation that
            // the transport is readable (non-transport message) but sending is critical, may occur
            // after we sent KEX_INIT and the other side is still sending regular data.
            // At this point we return `Pending` out of thin air. It is required that a transport
            // user also tries to read and consume in order to ensure progress!
            Condition::Writable if !self.kex.is_sending_critical() => Poll::Ready(Ok(())),
            Condition::Writable => Poll::Pending,
            // Condition is fulfilled if kex is not active. If transport is readable this means
            // that it must have blocked on sending kex messages in order to finish kex. Being
            // here is invalid state. If transport is not readable it is safe to return `Pending`.
            Condition::KexComplete if !self.kex.is_active() => Poll::Ready(Ok(())),
            Condition::KexComplete if readable => Poll::Ready(Err(TransportError::InvalidState)),
            Condition::KexComplete => Poll::Pending,
        }
    }

    /// Poll sending a message.
    ///
    /// Returns `Pending` if the sender does not have enough space and needs to be flushed first.
    /// Resets the alive timer on success.
    fn tx_msg<Msg: SshEncode>(
        &mut self,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let size = SshCodec::size(msg).ok_or(TransportError::InvalidEncoding)?;
        let buf = ready!(self.trx.tx_alloc(cx, size))?;
        let mut e = RefEncoder::new(buf);
        e.push(msg).ok_or(TransportError::InvalidEncoding)?;
        self.tx_commit()?;
        Poll::Ready(Ok(()))
    }
}

impl<S: Socket> Transport for DefaultTransport<S> {
    fn rx_peek(&mut self, cx: &mut Context) -> Poll<Result<&[u8], TransportError>> {
        ready!(self.poll(cx, Condition::Readable))?;
        self.trx.rx_peek(cx)
    }

    fn rx_consume(&mut self) -> Result<(), TransportError> {
        self.trx.rx_consume()
    }

    fn tx_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        ready!(self.poll(cx, Condition::Writable))?;
        self.trx.tx_alloc(cx, len)
    }

    fn tx_commit(&mut self) -> Result<(), TransportError> {
        self.trx.tx_commit()
    }

    fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.trx.tx_flush(cx)
    }

    fn tx_disconnect(&mut self, cx: &mut Context, reason: DisconnectReason) {
        let msg = MsgDisconnect::new(reason);
        let _ = self.tx_msg(cx, &msg);
        let _ = self.tx_flush(cx);
    }

    fn session_id(&self) -> Result<&SessionId, TransportError> {
        self.kex.session_id()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Condition {
    /// Ready for transmission of non-transport message
    Writable,
    /// Ready for reception of non-transport message
    Readable,
    /// Kex is complete (sent and received MSG_NEWKEYS)
    KexComplete,
}
