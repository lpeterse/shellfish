use super::transceiver::*;
use super::*;
use crate::util::socket::Socket;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::{sleep, Instant, Sleep};

/// Implements the transport layer as described in RFC 4253.
///
/// This structure is polymorphic in the socket type (most likely `TcpStream` but other types are
/// used for testing).
#[derive(Debug)]
pub struct DefaultTransport<S: Socket = TcpStream> {
    config: Arc<TransportConfig>,
    trx: Transceiver<S>,
    kex: Box<dyn Kex>,
    kex_rx_critical: bool,
    kex_tx_critical: bool,
    /// Rekeying timeout (reset after successful kex)
    kex_next_at_timeout: Pin<Box<Sleep>>,
    /// Rekeying threshold (updated on kex init)
    kex_next_at_tx_bytes: u64,
    /// Rekeying threshold (updated on kex init)
    kex_next_at_rx_bytes: u64,
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
        let kex = ClientKex::new(config, host_verifier, host_name, host_port, id);
        let kex = Box::new(kex);
        let mut transport = Self::new(config, trx, kex);
        poll_fn(|cx| transport.poll_first_kex(cx)).await?;
        Ok(transport)
    }

    /// Create a new transport acting as server.
    ///
    /// The initial key exchange has been completed successfully when function returns.
    pub async fn accept(
        config: &Arc<TransportConfig>,
        socket: S,
        agent: &Arc<dyn AuthAgent>,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(&config, socket);
        trx.tx_id(&config.identification).await?;
        let id = trx.rx_id(false).await?;
        let kex = ServerKex::new(config, agent, id);
        let kex = Box::new(kex);
        let mut transport = Self::new(config, trx, kex);
        poll_fn(|cx| transport.poll_first_kex(cx)).await?;
        Ok(transport)
    }

    fn new(config: &Arc<TransportConfig>, trx: Transceiver<S>, kex: Box<dyn Kex>) -> Self {
        Self {
            config: config.clone(),
            trx,
            kex,
            kex_rx_critical: true,
            kex_tx_critical: true,
            kex_next_at_timeout: Box::pin(sleep(config.kex_interval_duration)),
            kex_next_at_tx_bytes: config.kex_interval_bytes,
            kex_next_at_rx_bytes: config.kex_interval_bytes,
        }
    }

    fn init_kex(&mut self) {
        let txb = self.trx.tx_bytes();
        let rxb = self.trx.rx_bytes();
        let deadline = Instant::now() + self.config.kex_interval_duration;
        self.kex_next_at_timeout.as_mut().reset(deadline);
        self.kex_next_at_tx_bytes = txb + self.config.kex_interval_bytes;
        self.kex_next_at_rx_bytes = rxb + self.config.kex_interval_bytes;
        self.kex.init();
    }

    fn init_kex_if_necessary(&mut self, cx: &mut Context) {
        let txb = self.trx.tx_bytes();
        let rxb = self.trx.rx_bytes();
        let a = Future::poll(Pin::new(&mut self.kex_next_at_timeout), cx).is_ready();
        let b = txb > self.kex_next_at_tx_bytes;
        let c = rxb > self.kex_next_at_rx_bytes;
        if a || b || c {
            self.init_kex()
        }
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>> {
        // Process all incoming messages as long as they belong to the transport layer.
        while let Poll::Ready(buf) = self.trx.rx_peek(cx)? {
            match *buf.get(0).ok_or(TransportError::InvalidPacket)? {
                MsgDisconnect::NUMBER => {
                    // Try to interpret as MSG_DISCONNECT. If successful, convert it into an error
                    // and let the callee handle the termination.
                    let msg: MsgDisconnect = SshCodec::decode(buf)?;
                    log::debug!("Rx MSG_DISCONNECT: {:?}", msg.reason);
                    return Poll::Ready(Err(TransportError::DisconnectByPeer(msg.reason)));
                }
                MsgUnimplemented::NUMBER => {
                    // Try to interpret as MSG_UNIMPLEMENTED. Throw error as long as there is no
                    // need to handle it in a more sophisticated way.
                    let msg: MsgUnimplemented = SshCodec::decode(buf)?;
                    log::error!("Rx MSG_UNIMPLEMENTED: packet {}", msg.packet_number);
                    return Poll::Ready(Err(TransportError::InvalidState));
                }
                MsgIgnore::NUMBER => {
                    // Try to interpret as MSG_IGNORE. If successful, the message is (as the name
                    // suggests) just ignored. Ignore messages may be introduced any time to impede
                    // traffic analysis and for keep alive.
                    log::debug!("Rx MSG_IGNORE");
                    self.trx.rx_consume()?;
                }
                MsgDebug::NUMBER => {
                    // Try to interpret as MSG_DEBUG. If successful, log as debug and continue.
                    let msg: MsgDebug = SshCodec::decode(buf)?;
                    log::debug!("Rx MSG_DEBUG: {:?}", msg.message);
                    self.trx.rx_consume()?;
                }
                MsgKexInit::<String>::NUMBER => {
                    // Try to interpret as MSG_KEX_INIT. If successful, pass it to the kex handler.
                    // Unless the protocol is violated, kex is in progress afterwards (if not already).
                    log::debug!("Rx MSG_KEX_INIT");
                    let msg: MsgKexInit = SshCodec::decode(buf)?;
                    self.kex.push_init(msg)?;
                    self.trx.rx_consume()?;
                    self.kex_rx_critical = true;
                }
                MsgKexEcdhInit::NUMBER => {
                    log::debug!("Rx MSG_ECDH_INIT");
                    let msg: MsgKexEcdhInit = SshCodec::decode(buf)?;
                    self.kex.push_ecdh_init(msg)?;
                    self.trx.rx_consume()?;
                }
                MsgKexEcdhReply::NUMBER => {
                    log::debug!("Rx MSG_ECDH_REPLY");
                    let msg: MsgKexEcdhReply = SshCodec::decode(buf)?;
                    self.kex.push_ecdh_reply(msg)?;
                    self.trx.rx_consume()?;
                }
                MsgNewKeys::NUMBER => {
                    log::debug!("Rx MSG_NEWKEYS");
                    let cipher = self.kex.push_new_keys()?;
                    self.trx.rx_cipher().update(cipher)?;
                    self.trx.rx_consume()?;
                    self.kex_rx_critical = false;
                }
                _ => {
                    if self.kex_rx_critical {
                        return Poll::Ready(Err(TransportError::InvalidState));
                    } else {
                        break;
                    }
                }
            }
        }

        // Poll the kex machine: It only returns [Poll::Pending] if it is in a critical stage
        // like signing and we shall not proceed with anything else until this unblocks.
        // In this case we escalate `pending` all the way up.
        self.init_kex_if_necessary(cx);
        let queue = ready!(self.kex.poll(cx))?;
        let mut flush = false;
        while let Some(ref x) = queue.front() {
            match x {
                KexMessage::Init(x) => {
                    ready!(Self::tx_msg(&mut self.trx, cx, x.as_ref()))?;
                    log::debug!("Tx MSG_KEX_INIT");
                    self.kex_tx_critical = true;
                }
                KexMessage::EcdhInit(x) => {
                    ready!(Self::tx_msg(&mut self.trx, cx, x.as_ref()))?;
                    log::debug!("Tx MSG_ECDH_INIT");
                }
                KexMessage::EcdhReply(x) => {
                    ready!(Self::tx_msg(&mut self.trx, cx, x.as_ref()))?;
                    log::debug!("Tx MSG_ECDH_REPLY");
                }
                KexMessage::NewKeys(_) => {
                    ready!(Self::tx_msg(&mut self.trx, cx, &MsgNewKeys))?;
                    log::debug!("Tx MSG_NEWKEYS");
                    self.kex_tx_critical = false;
                }
            }
            if let Some(KexMessage::NewKeys(x)) = queue.pop_front() {
                self.trx.tx_cipher().update(x)?;
            }
            flush = true;
        }

        // Flush the transceiver in case any kex messages have been added that require
        // actual transmission for generating progress.
        // Flushing might block: In this case we escalate as the kex won't finish until all kex
        // messages have been transmitted to the peer.
        if flush {
            ready!(self.trx.tx_flush(cx))?;
        }

        // Being here means that all transport messages have been consumed, all kex messages have
        // been sent and flushed and that it is guaranteed that there is either an inbound
        // non-transport message available or the socket has been polled for reading and the task
        // will be woken as soon as more inbound data arrives.
        Poll::Ready(Ok(self.trx.rx_next()))
    }

    fn poll_first_kex(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        let _ = ready!(self.poll(cx))?;
        if self.kex.session_id().is_some() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    /// Poll sending a message.
    ///
    /// Returns `Pending` if the sender does not have enough space and needs to be flushed first.
    fn tx_msg<Msg: SshEncode>(
        trx: &mut Transceiver<S>,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let size = SshCodec::size(msg)?;
        let buf = ready!(trx.tx_alloc(cx, size))?;
        let mut e = RefEncoder::new(buf);
        e.push(msg).ok_or(SshCodecError::EncodingFailed)?;
        trx.tx_commit()?;
        Poll::Ready(Ok(()))
    }
}

impl<S: Socket> Transport for DefaultTransport<S> {
    fn poll_next(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>> {
        self.poll(cx)
    }

    fn consume_next(&mut self) -> Result<(), TransportError> {
        self.trx.rx_consume()
    }

    fn poll_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        if self.kex_tx_critical {
            Poll::Pending
        } else {
            self.trx.tx_alloc(cx, len)
        }
    }

    fn commit_alloc(&mut self) -> Result<(), TransportError> {
        self.trx.tx_commit()
    }

    fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.trx.tx_flush(cx)
    }

    fn session_id(&self) -> Result<&Secret, TransportError> {
        self.kex.session_id().ok_or(TransportError::InvalidState)
    }
}
