use super::super::keys::*;
use super::super::*;
use crate::ready;
use crate::util::BoxFuture;
use core::future::Future;
//use futures_timer::Delay;
use std::sync::Arc;
use tokio::time::{sleep, Sleep, Instant};
use std::pin::Pin;

/// The client side state machine for key exchange.
#[derive(Debug)]
pub struct ClientKex {
    config: Arc<TransportConfig>,
    /// Mutable state (when kex in progress)
    state: Option<Box<State>>,
    /// Remote identification string
    host_id: Identification,
    /// Remote hostname for host key verification
    host_name: String,
    /// Remote port
    host_port: u16,
    /// Host identity verifier
    host_verifier: Arc<dyn HostVerifier>,
    /// Rekeying timeout (reset after successful kex)
    next_at: Pin<Box<Sleep>>,
    /// Rekeying threshold (updated after successful kex)
    next_at_bytes_sent: u64,
    /// Rekeying threshold (updates after successful kex)
    next_at_bytes_received: u64,
    /// Session id (only after after initial kex, constant afterwards)
    session_id: Option<SessionId>,
}

impl ClientKex {
    pub fn new(
        config: &Arc<TransportConfig>,
        host_id: Identification<String>,
        host_name: &str,
        host_port: u16,
        host_verifier: &Arc<dyn HostVerifier>,
    ) -> Self {
        Self {
            config: config.clone(),
            state: None,
            host_id,
            host_name: host_name.into(),
            host_port,
            host_verifier: host_verifier.clone(),
            next_at: Box::pin(sleep(config.kex_interval_duration)),
            next_at_bytes_sent: config.kex_interval_bytes,
            next_at_bytes_received: config.kex_interval_bytes,
            session_id: None,
        }
    }
}

struct State {
    cookie: KexCookie,
    secret: <X25519 as EcdhAlgorithm>::EphemeralSecret,
    cipher: Option<(CipherConfig, CipherConfig)>,
    init_tx: bool,
    init_rx: Option<MsgKexInit>,
    ecdh_tx: bool,
    ecdh_rx: Option<Result<(), BoxFuture<Result<(), HostVerificationError>>>>,
    newkeys_tx: bool,
    newkeys_rx: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            cookie: KexCookie::random(),
            secret: X25519::new(),
            cipher: None,
            init_tx: false,
            init_rx: None,
            ecdh_tx: false,
            ecdh_rx: None,
            newkeys_tx: false,
            newkeys_rx: false,
        }
    }

    fn poll_host_key_verified(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<Option<()>, TransportError>> {
        if let Some(ref mut hkv) = self.ecdh_rx {
            match hkv {
                Ok(()) => Poll::Ready(Ok(Some(()))),
                Err(ref mut hkv) => {
                    ready!(Pin::new(hkv).poll(cx)).map_err(TransportError::InvalidIdentity)?;
                    self.ecdh_rx = Some(Ok(()));
                    return Poll::Ready(Ok(Some(())));
                }
            }
        } else {
            Poll::Ready(Ok(None))
        }
    }
}

impl Kex for ClientKex {
    fn is_active(&self) -> bool {
        self.state.is_some()
    }

    fn is_sending_critical(&self) -> bool {
        if let Some(ref state) = self.state {
            state.init_tx && !state.newkeys_tx
        } else {
            false
        }
    }

    fn is_receiving_critical(&self) -> bool {
        if let Some(ref state) = self.state {
            state.init_rx.is_some() && !state.newkeys_rx
        } else {
            false
        }
    }

    fn init(&mut self, tx: u64, rx: u64) {
        if self.state.is_none() {
            let deadline = Instant::now() + self.config.kex_interval_duration;
            self.next_at.as_mut().reset(deadline);
            self.next_at_bytes_sent = tx + self.config.kex_interval_bytes;
            self.next_at_bytes_received = rx + self.config.kex_interval_bytes;
            self.state = Some(Box::new(State::new()))
        }
    }

    fn init_if_necessary(&mut self, cx: &mut Context, tx: u64, rx: u64) {
        if self.state.is_none() {
            let a = Future::poll(Pin::new(&mut self.next_at), cx).is_ready();
            let b = tx > self.next_at_bytes_sent;
            let c = rx > self.next_at_bytes_received;
            if a || b || c {
                self.init(tx, rx);
            }
        }
    }

    fn peek_init(&mut self, _cx: &mut Context) -> Option<MsgKexInit<&'static str>> {
        if let Some(ref state) = self.state {
            if !state.init_tx {
                let msg = MsgKexInit::new_from_config(state.cookie, &self.config);
                return Some(msg);
            }
        }
        None
    }

    fn push_init_tx(&mut self) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if !state.init_tx {
                state.init_tx = true;
                return Ok(());
            }
        }
        Err(TransportError::InvalidState)
    }

    fn push_init_rx(&mut self, tx: u64, rx: u64, msg: MsgKexInit) -> Result<(), TransportError> {
        self.init(tx, rx);
        if let Some(ref mut state) = self.state {
            if state.init_rx.is_none() {
                state.init_rx = Some(msg);
                return Ok(());
            }
        }
        Err(TransportError::InvalidState)
    }

    fn peek_ecdh_init(
        &mut self,
        _cx: &mut Context,
    ) -> Result<Option<MsgKexEcdhInit<X25519>>, TransportError> {
        if let Some(ref mut state) = self.state {
            if state.init_tx && !state.ecdh_tx {
                if let Some(ref remote) = state.init_rx {
                    let ka = intersection(&self.config.kex_algorithms, &KEX_ALGORITHMS[..]);
                    let ka = common(&ka, &remote.kex_algorithms);
                    if ka == Some(Curve25519Sha256::NAME)
                        || ka == Some(Curve25519Sha256AtLibsshDotOrg::NAME)
                    {
                        let msg = MsgKexEcdhInit::new(X25519::public(&state.secret));
                        return Ok(Some(msg));
                    }
                    return Err(TransportError::NoCommonKexAlgorithm);
                }
            }
        }
        Ok(None)
    }

    fn push_ecdh_init_tx(&mut self) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if state.init_tx && !state.ecdh_tx {
                state.ecdh_tx = true;
                return Ok(());
            }
        }
        Err(TransportError::InvalidState)
    }

    fn push_ecdh_init_rx(&mut self, _msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        Err(TransportError::InvalidState)
    }

    fn peek_ecdh_reply(
        &mut self,
        _cx: &mut Context,
    ) -> Result<Option<MsgKexEcdhReply<X25519>>, TransportError> {
        Ok(None)
    }

    fn push_ecdh_reply_tx(&mut self) -> Result<(), TransportError> {
        Err(TransportError::InvalidState)
    }

    fn push_ecdh_reply_rx(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if state.ecdh_tx && state.ecdh_rx.is_none() {
                if let Some(ref remote) = state.init_rx {
                    let local = MsgKexInit::new_from_config(state.cookie, &self.config);
                    // Compute the DH shared secret (create a new placeholder while
                    // the actual secret gets consumed in the operation).
                    let dh_secret = std::mem::replace(&mut state.secret, X25519::new());
                    let dh_public = X25519::public(&dh_secret);
                    let k = X25519::diffie_hellman(dh_secret, &msg.dh_public);
                    //let k = X25519::secret_as_ref(&k);
                    // Compute the exchange hash over the data exchanged so far.
                    let h: SessionId = KexEcdhHash::<X25519> {
                        client_id: &self.config.identification,
                        server_id: &self.host_id,
                        client_kex_init: &local,
                        server_kex_init: &remote,
                        server_host_key: &msg.host_key,
                        dh_client_key: &dh_public,
                        dh_server_key: &msg.dh_public,
                        dh_secret: X25519::secret_as_ref(&k),
                    }
                    .sha256();
                    // Verify the host key signature
                    msg.signature
                        .verify(&msg.host_key, h.as_ref())
                        .map_err(|_| TransportError::InvalidSignature)?;
                    // The session id is only computed during first kex and constant afterwards
                    let sid = self.session_id.get_or_insert_with(|| h.clone());
                    let algos = AlgorithmAgreement::agree(&local, &remote)?;
                    let keys_c2s = KeyStream::new(
                        KeyDirection::ClientToServer,
                        KeyAlgorithm::Sha256,
                        k.to_bytes(),
                        h.clone(),
                        sid.clone(),
                    );
                    let keys_s2c = KeyStream::new(
                        KeyDirection::ServerToClient,
                        KeyAlgorithm::Sha256,
                        k.to_bytes(),
                        h.clone(),
                        sid.clone(),
                    );
                    let enc = CipherConfig {
                        ea: algos.ea_c2s,
                        ca: algos.ca_c2s,
                        ma: algos.ma_c2s,
                        ke: keys_c2s,
                    };
                    let dec = CipherConfig {
                        ea: algos.ea_s2c,
                        ca: algos.ca_s2c,
                        ma: algos.ma_s2c,
                        ke: keys_s2c,
                    };
                    let fut = self
                        .host_verifier
                        .verify(&self.host_name, self.host_port, &msg.host_key);
                    state.ecdh_rx = Some(Err(fut));
                    state.cipher = Some((enc, dec));
                    return Ok(());
                }
            }
        }
        Err(TransportError::InvalidState)
    }

    fn poll_new_keys_tx(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<Option<EncryptionConfig>, TransportError>> {
        if let Some(ref mut state) = self.state {
            if let Some(()) = ready!(state.poll_host_key_verified(cx))? {
                return if let Some((ref enc, _)) = state.cipher {
                    Poll::Ready(Ok(Some(enc.clone())))
                } else {
                    Poll::Ready(Err(TransportError::InvalidState))
                };
            }
        }
        Poll::Ready(Ok(None))
    }

    fn poll_new_keys_rx(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<EncryptionConfig, TransportError>> {
        if let Some(ref mut state) = self.state {
            if let Some(()) = ready!(state.poll_host_key_verified(cx))? {
                return if let Some((_, ref dec)) = state.cipher {
                    Poll::Ready(Ok(dec.clone()))
                } else {
                    Poll::Ready(Err(TransportError::InvalidState))
                };
            }
        }
        Poll::Ready(Err(TransportError::InvalidState))
    }

    fn push_new_keys_tx(&mut self) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if let Some(Ok(())) = state.ecdh_rx {
                if !state.newkeys_tx {
                    state.newkeys_tx = true;
                    if state.newkeys_rx {
                        self.state = None;
                    }
                    return Ok(());
                }
            }
        }
        Err(TransportError::InvalidState)
    }

    fn push_new_keys_rx(&mut self) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if let Some(Ok(())) = state.ecdh_rx {
                if !state.newkeys_rx {
                    state.newkeys_rx = true;
                    if state.newkeys_tx {
                        self.state = None;
                    }
                    return Ok(());
                }
            }
        }
        Err(TransportError::InvalidState)
    }

    fn session_id(&self) -> Result<&SessionId, TransportError> {
        self.session_id.as_ref().ok_or(TransportError::InvalidState)
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "State {{ ... }}")
    }
}
