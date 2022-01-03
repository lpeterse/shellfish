pub(crate) mod config;
pub(crate) mod crypto;
pub(crate) mod disconnect;
pub(crate) mod error;
pub(crate) mod ident;
pub(crate) mod kex;
pub(crate) mod keys;
pub(crate) mod msg;
pub(crate) mod trx;

pub(crate) use self::crypto::*;
pub(crate) use self::kex::*;
pub(crate) use self::msg::*;

pub use self::config::TransportConfig;
pub use self::disconnect::DisconnectReason;
pub use self::error::TransportError;
pub use self::ident::Identification;

use self::trx::*;
use crate::agent::AuthAgent;
use crate::host::HostVerifier;
use crate::ready;
use crate::util::codec::SshCodec;
use crate::util::codec::SshDecode;
use crate::util::codec::SshEncode;
use crate::util::poll_fn;
use crate::util::secret::Secret;
use crate::util::socket::Socket;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::time::{sleep, Instant, Sleep};

/// Implements the transport layer as described in RFC 4253.
#[derive(Debug)]
pub struct Transport {
    config: Arc<TransportConfig>,
    trx: Transceiver,
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

impl Transport {
    /// Create a new transport acting as client.
    ///
    /// The initial key exchange has completed successfully when function returns.
    pub async fn connect<S: Socket>(
        socket: S,
        config: &Arc<TransportConfig>,
        host_verifier: &Arc<dyn HostVerifier>,
        host_name: &str,
        host_port: u16,
        service: &str,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(config, socket);
        trx.tx_id(&config.identification).await?;
        let id = trx.rx_id(true).await?;
        let kex = ClientKex::new(config, host_verifier, host_name, host_port, id);
        let mut t = Self::new(config, trx, kex);
        t.send(&MsgServiceRequest(service)).await?;
        t.flush().await?;
        log::debug!("Tx MSG_SERVICE_REQUEST");
        t.receive::<MsgServiceAccept>().await?;
        log::debug!("Rx MSG_SERVICE_ACCEPT");
        Ok(t)
    }

    /// Create a new transport acting as server.
    ///
    /// The initial key exchange has completed successfully when function returns.
    pub async fn accept<S: Socket>(
        socket: S,
        config: &Arc<TransportConfig>,
        agent: &Arc<dyn AuthAgent>,
        service: &str,
    ) -> Result<Self, TransportError> {
        let mut trx = Transceiver::new(&config, socket);
        trx.tx_id(&config.identification).await?;
        let id = trx.rx_id(false).await?;
        let kex = ServerKex::new(config, agent, id);
        let mut t = Self::new(config, trx, kex);
        let msg = t.receive::<MsgServiceRequest>().await?;
        log::debug!("Rx MSG_SERVICE_REQUEST: {}", msg.0);
        if msg.0 == service {
            t.send(&MsgServiceAccept(service)).await?;
            log::debug!("Rx MSG_SERVICE_ACCEPT: {}", service);
            t.flush().await?;
            Ok(t)
        } else {
            let reason = DisconnectReason::SERVICE_NOT_AVAILABLE;
            t.send(&MsgDisconnect::new(reason)).await?;
            log::debug!("Rx MSG_DISCONNECT: {}", reason);
            t.flush().await?;
            Err(TransportError::InvalidServiceRequest(msg.0))
        }
    }

    /// Send a message.
    ///
    /// This method shall only be used to send non-transport messages.
    ///
    /// Sending a message only enqueues it for transmission, but it is not guaranteed that it has
    /// actually been transmitted when this function returns. Use [flush](Self::flush) in order
    /// to ensure actual transmission (but use it wisely).
    pub async fn send<M: Message + SshEncode>(&mut self, msg: &M) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_send(cx, msg)).await
    }

    /// Receive a message.
    ///
    /// Only non-transport messages will be received (others are dispatched internally).
    ///
    /// The receive call blocks until either the next non-transport message arrives or an error
    /// occurs.
    pub async fn receive<M: SshDecode>(&mut self) -> Result<M, TransportError> {
        poll_fn(|cx| self.poll_receive(cx)).await
    }

    /// Flush all output buffers.
    ///
    /// When the function returns without an error it is guaranteed that all messages that have
    /// previously been enqueued with [send](Self::send) have been transmitted and all output
    /// buffers are empty. Of course, this does not imply anything about successful reception of
    /// those messages.
    ///
    /// Do not flush too deliberately! The transport will automatically transmit the output buffers
    /// when you continue filling them by sending messages. Flush shall rather be used when
    /// you sent something like a request and need to make sure it has been transmitted before
    /// starting to wait for the response.
    pub async fn flush(&mut self) -> Result<(), TransportError> {
        poll_fn(|cx| self.poll_flush(cx)).await
    }

    /// Like [send](Self::send) but low-level.
    pub fn poll_send<M: Message + SshEncode>(
        &mut self,
        cx: &mut Context,
        msg: &M,
    ) -> Poll<Result<(), TransportError>> {
        if Self::forbidden_while_kex(M::NUMBER) && self.kex_tx_critical {
            // Message must not be sent right now. Need to finish kex first..
            let unconsumed = ready!(self.poll_receive_buf(cx))?.is_some();
            // Most likely the `ready!` has returned pending. Otherwise we check again..
            if self.kex_tx_critical {
                // Kex still not finished..
                return if unconsumed {
                    // Unconsumed non-transport inbound message makes it impossible to finish
                    // kex. This is considered an error condition that should have been avoided
                    // by calling [poll_receive_buf] before. We must not return Pending here
                    // as this would lead to the task never waking up again.
                    Poll::Ready(Err(TransportError::InvalidMessageKexCritical))
                } else {
                    Poll::Pending // safe as to definition of [poll_receive_buf]
                };
            }
        }
        Self::poll_send_trx(&mut self.trx, cx, msg)
    }

    /// Like [receive](Self::poll) but low-level.
    pub fn poll_receive<M: SshDecode>(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<M, TransportError>> {
        if let Some(buf) = ready!(self.poll_receive_buf(cx))? {
            let msg = SshCodec::decode(buf)?;
            self.consume_receive_buf()?;
            Poll::Ready(Ok(msg))
        } else {
            Poll::Pending // safe as to definition of [poll_receive_buf]
        }
    }

    /// Try to receive the next message as a buffer reference.
    ///
    /// This method does _not_ remove the message from the input queue. It is necessary to call
    /// [consume_buf](Self::consume_buf) once after the message has been processed (or it would be
    /// returned again on the next call).
    ///
    /// It is critical to dispatch and consume all messages in order to driver internal tasks like kex!
    ///
    /// - Returns `Pending` during key exchange (will unblock as soon as kex is complete).
    /// - Returns `Ready(Ok(Some(_)))` for each inbound non-transport layer message.
    /// - Returns `Ready(Ok(None))` if no message is available for now (MAY be escalated as [Poll::Pending]).
    pub fn poll_receive_buf(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<Option<&[u8]>, TransportError>> {
        log::debug!("poll_receive_buf");
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
                MsgEcdhInit::NUMBER => {
                    log::debug!("Rx MSG_ECDH_INIT");
                    let msg: MsgEcdhInit = SshCodec::decode(buf)?;
                    self.kex.push_ecdh_init(msg)?;
                    self.trx.rx_consume()?;
                }
                MsgEcdhReply::NUMBER => {
                    log::debug!("Rx MSG_ECDH_REPLY");
                    let msg: MsgEcdhReply = SshCodec::decode(buf)?;
                    self.kex.push_ecdh_reply(msg)?;
                    self.trx.rx_consume()?;
                }
                MsgNewkeys::NUMBER => {
                    log::debug!("Rx MSG_NEWKEYS");
                    let cipher = self.kex.push_new_keys()?;
                    self.trx.rx_cipher().update(cipher)?;
                    self.trx.rx_consume()?;
                    self.kex_rx_critical = false;
                }
                n => {
                    if Self::forbidden_while_kex(n) && self.kex_rx_critical {
                        return Poll::Ready(Err(TransportError::InvalidMessageKexCritical));
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
                    ready!(Self::poll_send_trx(&mut self.trx, cx, x.as_ref()))?;
                    log::debug!("Tx MSG_KEX_INIT");
                    self.kex_tx_critical = true;
                }
                KexMessage::EcdhInit(x) => {
                    ready!(Self::poll_send_trx(&mut self.trx, cx, x.as_ref()))?;
                    log::debug!("Tx MSG_ECDH_INIT");
                }
                KexMessage::EcdhReply(x) => {
                    ready!(Self::poll_send_trx(&mut self.trx, cx, x.as_ref()))?;
                    log::debug!("Tx MSG_ECDH_REPLY");
                }
                KexMessage::NewKeys(_) => {
                    ready!(Self::poll_send_trx(&mut self.trx, cx, &MsgNewkeys))?;
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
        // Flushing might block: In this case we escalate pending as the kex won't finish
        // until all kex messages have been transmitted to the peer.
        if flush {
            ready!(self.trx.tx_flush(cx))?;
        }

        // Being here means that all transport messages have been consumed, all kex messages have
        // been sent and flushed and that it is guaranteed that there is either an inbound
        // non-transport message available or the socket has been polled for reading and the task
        // will be woken as soon as more inbound data arrives.
        Poll::Ready(Ok(self.trx.rx_next()))
    }

    /// Consume the current receive buffer (only after [poll_receive_buf](Self::poll_receive_buf)).
    ///
    /// The message must have been decoded and processed before.
    pub fn consume_receive_buf(&mut self) -> Result<(), TransportError> {
        self.trx.rx_consume()
    }

    /// Like [flush](Self::flush) but low-level.
    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        self.trx.tx_flush(cx)
    }

    /// Return the connection's session id.
    ///
    /// The session id is a result of the initial key exchange.
    /// It is static for the whole lifetime of the connection.
    pub fn session_id(&self) -> &Secret {
        // First kex is guaranteed to be completed after [Self::accept] and [Self::connect]
        self.kex
            .session_id()
            .expect("called before first kex complete")
    }

    // ---------------------------------------------------------------------------------------------
    //  PRIVATE METHODS
    // ---------------------------------------------------------------------------------------------

    fn new(config: &Arc<TransportConfig>, trx: Transceiver, kex: Box<dyn Kex>) -> Self {
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

    fn poll_send_trx<Msg: SshEncode>(
        trx: &mut Transceiver,
        cx: &mut Context,
        msg: &Msg,
    ) -> Poll<Result<(), TransportError>> {
        let size = SshCodec::size(msg)?;
        let buf = ready!(trx.tx_alloc(cx, size))?;
        SshCodec::encode_into(msg, buf)?;
        trx.tx_commit()?;
        Poll::Ready(Ok(()))
    }

    fn forbidden_while_kex(msg_number: u8) -> bool {
        msg_number > 49 || msg_number == 5 || msg_number == 6
    }
}
