use super::super::config::*;
use super::kex::*;
use crate::algorithm::kex::*;

use async_std::future::Future;

/// The client side state machine for key exchange.
pub struct ClientKex {
    config: Arc<TransportConfig>,
    /// Host key verifier
    verifier: Arc<dyn HostKeyVerifier>,
    /// Mutable state (when kex in progress)
    state: Option<Box<State>>,
    /// Remote identification string
    remote_id: Identification,
    /// Remote name (hostname) for host key verification
    remote_name: String,
    /// Rekeying timeout (reset after successful kex)
    next_at: Delay,
    /// Rekeying threshold (updated after successful kex)
    next_at_bytes_sent: u64,
    /// Rekeying threshold (updates after successful kex)
    next_at_bytes_received: u64,
    /// Session id (only after after initial kex, constant afterwards)
    session_id: SessionId,
}

impl ClientKex {
    pub fn new(
        config: &Arc<TransportConfig>,
        verifier: &Arc<dyn HostKeyVerifier>,
        remote_id: Identification<String>,
        remote_name: String,
    ) -> Self {
        Self {
            config: config.clone(),
            verifier: verifier.clone(),
            state: None,
            remote_id,
            remote_name,
            next_at: Delay::new(config.kex_interval_duration),
            next_at_bytes_sent: config.kex_interval_bytes,
            next_at_bytes_received: config.kex_interval_bytes,
            session_id: SessionId::default(),
        }
    }
}

struct State {
    cookie: KexCookie,
    secret: <X25519 as EcdhAlgorithm>::EphemeralSecret,
    cipher: Option<(EncryptionConfig, DecryptionConfig)>,
    init_tx: bool,
    init_rx: Option<MsgKexInit>,
    ecdh_tx: bool,
    ecdh_rx: Option<Result<(), VerificationFuture>>,
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
}

impl State {
    fn poll_host_key_verified(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        if let Some(ref mut hkv) = self.ecdh_rx {
            if let Err(ref mut hkv) = hkv {
                let e = TransportError::HostKeyUnverifiable;
                ready!(Pin::new(hkv).poll(cx)).map_err(|_| e)?;
                self.ecdh_rx = Some(Ok(()))
            }
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
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
            self.next_at.reset(self.config.kex_interval_duration);
            self.next_at_bytes_sent = tx + self.config.kex_interval_bytes;
            self.next_at_bytes_received = rx + self.config.kex_interval_bytes;
            self.state = Some(Box::new(State::new()))
        }
    }

    fn poll_init(
        &mut self,
        cx: &mut Context,
        tx: u64,
        rx: u64,
    ) -> Poll<Result<MsgKexInit<&'static str>, TransportError>> {
        // Determine whether kex is required according to timeout or traffic.
        // This might evaluate to true even if kex is already in progress, but is harmless.
        let a = Pin::new(&mut self.next_at).poll(cx).is_ready();
        let b = tx > self.next_at_bytes_sent;
        let c = rx > self.next_at_bytes_received;
        if a || b || c {
            self.init(tx, rx);
        }
        if let Some(ref state) = self.state {
            if !state.init_tx {
                let msg = MsgKexInit::new_from_config(state.cookie, &self.config);
                return Poll::Ready(Ok(msg));
            }
        }
        Poll::Pending
    }

    fn push_init_tx(&mut self) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if !state.init_tx {
                state.init_tx = true;
                return Ok(());
            }
        }
        Err(TransportError::ProtocolError)
    }

    fn push_init_rx(&mut self, msg: MsgKexInit) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if state.init_rx.is_none() {
                state.init_rx = Some(msg);
                return Ok(());
            }
        }
        Err(TransportError::ProtocolError)
    }

    fn poll_ecdh_init(
        &mut self,
        _cx: &mut Context,
    ) -> Poll<Result<MsgKexEcdhInit<X25519>, TransportError>> {
        if let Some(ref mut state) = self.state {
            if state.init_tx && !state.ecdh_tx {
                if let Some(ref remote) = state.init_rx {
                    use crate::algorithm::KEX_ALGORITHMS;
                    let ka = intersection(&self.config.kex_algorithms, &KEX_ALGORITHMS[..]);
                    let ka = common(&ka, &remote.kex_algorithms);
                    if ka == Some(Curve25519Sha256::NAME)
                        || ka == Some(Curve25519Sha256AtLibsshDotOrg::NAME)
                    {
                        let msg = MsgKexEcdhInit::new(X25519::public(&state.secret));
                        return Poll::Ready(Ok(msg));
                    }
                    let e = TransportError::NoCommonKexAlgorithm;
                    return Poll::Ready(Err(e));
                }
            }
        }
        Poll::Pending
    }

    fn push_ecdh_init_tx(&mut self) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if state.init_tx && !state.ecdh_tx {
                state.ecdh_tx = true;
                return Ok(());
            }
        }
        Err(TransportError::ProtocolError)
    }

    fn push_ecdh_init_rx(&mut self, _msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        Err(TransportError::ProtocolError)
    }

    fn poll_ecdh_reply(
        &mut self,
        _cx: &mut Context,
    ) -> Poll<Result<MsgKexEcdhReply<X25519>, TransportError>> {
        Poll::Pending
    }

    fn push_ecdh_reply_tx(&mut self) -> Result<(), TransportError> {
        Err(TransportError::ProtocolError)
    }

    fn push_ecdh_reply_rx(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        if let Some(ref mut state) = self.state {
            if state.ecdh_tx && state.ecdh_rx.is_none() {
                if let Some(ref remote) = state.init_rx {
                    let local = MsgKexInit::new_from_config(state.cookie, &self.config);
                    // Compute the DH shared secret (create a new placeholder while
                    // the actual secret get consumed in the operation).
                    let dh_secret = std::mem::replace(&mut state.secret, X25519::new());
                    let dh_public = X25519::public(&dh_secret);
                    let k = X25519::diffie_hellman(dh_secret, &msg.dh_public);
                    let k = X25519::secret_as_ref(&k);
                    // Compute the exchange hash over the data exchanged so far.
                    let h: [u8; 32] = KexEcdhHash::<X25519> {
                        client_identification: &self.config.identification,
                        server_identification: &self.remote_id,
                        client_kex_init: &local,
                        server_kex_init: &remote,
                        server_host_key: &msg.host_key,
                        dh_client_key: &dh_public,
                        dh_server_key: &msg.dh_public,
                        dh_secret: k,
                    }
                    .sha256();
                    // Verify the host key signature
                    msg.signature
                        .verify(&msg.host_key.public_key(), &h[..])
                        .ok_or(TransportError::InvalidSignature)?;
                    // The session id is only computed during first kex and constant afterwards
                    self.session_id.update(h);
                    let algos = AlgorithmAgreement::agree(&local, &remote)?;
                    let keys = KeyStreams::new_sha256(k, &h, &self.session_id);
                    let enc = CipherConfig {
                        ea: algos.ea_c2s,
                        ca: algos.ca_c2s,
                        ma: algos.ma_c2s,
                        ke: keys.c(),
                    };
                    let dec = CipherConfig {
                        ea: algos.ea_s2c,
                        ca: algos.ca_s2c,
                        ma: algos.ma_s2c,
                        ke: keys.d(),
                    };
                    let fut = self.verifier.verify(&self.remote_name, &msg.host_key);
                    state.ecdh_rx = Some(Err(fut));
                    state.cipher = Some((enc, dec));
                    return Ok(());
                }
            }
        }
        Err(TransportError::ProtocolError)
    }

    fn poll_new_keys_tx(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<EncryptionConfig, TransportError>> {
        if let Some(ref mut state) = self.state {
            ready!(state.poll_host_key_verified(cx))?;
            return if let Some((ref enc, _)) = state.cipher {
                Poll::Ready(Ok(enc.clone()))
            } else {
                Poll::Ready(Err(TransportError::ProtocolError))
            };
        }
        Poll::Pending
    }

    fn poll_new_keys_rx(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<EncryptionConfig, TransportError>> {
        if let Some(ref mut state) = self.state {
            ready!(state.poll_host_key_verified(cx))?;
            return if let Some((_, ref dec)) = state.cipher {
                Poll::Ready(Ok(dec.clone()))
            } else {
                Poll::Ready(Err(TransportError::ProtocolError))
            };
        }
        Poll::Pending
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
        Err(TransportError::ProtocolError)
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
        Err(TransportError::ProtocolError)
    }

    fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::*;

    #[test]
    fn test_client_kex_new() {
        let mut config = ClientConfig::default();
        config.kex_interval_bytes = 1234;
        config.kex_interval_duration = std::time::Duration::from_secs(1235);
        config.kex_algorithms = vec!["curve25519-sha256", "UNSUPPORTED"];
        config.mac_algorithms = vec!["UNSUPPORTED"];
        config.host_key_algorithms = vec!["UNSUPPORTED", "ssh-ed25519", "UNSUPPORTED"];
        config.encryption_algorithms = vec!["UNSUPPORTED", "chacha20-poly1305@openssh.com"];
        config.compression_algorithms = vec!["UNSUPPORTED", "none"];
        let f = |x: Vec<&'static str>| {
            x.iter()
                .filter(|a| *a != &"UNSUPPORTED")
                .map(|a| *a)
                .collect::<Vec<&str>>()
        };

        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("testing".into(), "".into());
        let remote_name: String = "hostname".into();
        let kex = ClientKex::new(&config, verifier, remote_id.clone(), remote_name.clone());

        assert_eq!(kex.local_id, *config.identification());
        assert_eq!(kex.remote_id, remote_id);
        assert_eq!(kex.remote_name, remote_name);
        assert_eq!(kex.interval_bytes, config.kex_interval_bytes);
        assert_eq!(kex.interval_duration, config.kex_interval_duration);
        assert_eq!(kex.next_at_bytes_sent, config.kex_interval_bytes);
        assert_eq!(kex.next_at_bytes_received, config.kex_interval_bytes);
        assert_eq!(kex.local_init.kex_algorithms, f(config.kex_algorithms));
        assert_eq!(
            kex.local_init.server_host_key_algorithms,
            f(config.host_key_algorithms)
        );
        assert_eq!(
            kex.local_init.mac_algorithms_client_to_server,
            f(config.mac_algorithms)
        );
        assert_eq!(
            kex.local_init.encryption_algorithms_client_to_server,
            f(config.encryption_algorithms)
        );
        assert_eq!(
            kex.local_init.compression_algorithms_client_to_server,
            f(config.compression_algorithms)
        );
        assert!(kex.state.is_none());
    }

    #[test]
    fn test_client_kex_is_active() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());
        assert!(!kex.is_active());
        kex.init();
        assert!(kex.is_active());
    }

    #[test]
    fn test_client_kex_is_sending_critical() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c.clone(), vec![], vec![], vec![], vec![], vec![]);
        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        // Shall not be critical after creation
        assert!(!kex.is_sending_critical());
        // Shall not be critical after init
        kex.state = Some(Box::new(State::Init(Init {
            local_init: None,
            remote_init: None,
        })));
        assert!(!kex.is_sending_critical());
        // Shall be critical after MSG_KEX_INIT was sent
        kex.state = Some(Box::new(State::Init(Init {
            local_init: Some(()),
            remote_init: None,
        })));
        assert!(kex.is_sending_critical());
        // Shall not be critical after MSG_KEX_INIT was received
        kex.state = Some(Box::new(State::Init(Init {
            local_init: None,
            remote_init: Some(ri),
        })));
        assert!(!kex.is_sending_critical());
        // Shall be critical while MGS_NEWKEYS neither sent nor received
        kex.state = Some(Box::new(State::NewKeys((cc.clone(), cc.clone()))));
        assert!(kex.is_sending_critical());
        // Shall be critical while MGS_NEWKEYS received but not sent
        kex.state = Some(Box::new(State::NewKeysReceived(cc.clone())));
        assert!(kex.is_sending_critical());
        // Shall not be critical after MSG_NEWKEYS was sent
        kex.state = Some(Box::new(State::NewKeysSent(cc)));
        assert!(!kex.is_sending_critical());
    }

    #[test]
    fn test_client_kex_is_receiving_critical() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c.clone(), vec![], vec![], vec![], vec![], vec![]);
        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        // Shall not be critical after creation
        assert!(!kex.is_receiving_critical());
        // Shall not be critical after init
        kex.state = Some(Box::new(State::Init(Init {
            local_init: None,
            remote_init: None,
        })));
        assert!(!kex.is_receiving_critical());
        // Shall not be critical after MSG_KEX_INIT was sent
        kex.state = Some(Box::new(State::Init(Init {
            local_init: Some(()),
            remote_init: None,
        })));
        assert!(!kex.is_receiving_critical());
        // Shall be critical after MSG_KEX_INIT was received
        kex.state = Some(Box::new(State::Init(Init {
            local_init: None,
            remote_init: Some(ri),
        })));
        assert!(kex.is_receiving_critical());
        // Shall be critical while MGS_NEWKEYS neither sent nor received
        kex.state = Some(Box::new(State::NewKeys((cc.clone(), cc.clone()))));
        assert!(kex.is_receiving_critical());
        // Shall not be critical while MGS_NEWKEYS received but not sent
        kex.state = Some(Box::new(State::NewKeysReceived(cc.clone())));
        assert!(!kex.is_receiving_critical());
        // Shall be critical after MSG_NEWKEYS was sent
        kex.state = Some(Box::new(State::NewKeysSent(cc)));
        assert!(kex.is_receiving_critical());
    }

    /// State shall be Init after init()
    #[test]
    fn test_client_kex_init_01() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        kex.init();
        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.local_init.is_none());
                    assert!(x.remote_init.is_none());
                }
                _ => assert!(false),
            },
        }
    }

    /// Shall not override kex state when kex is already active
    #[test]
    fn test_client_kex_init_02() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        kex.state = Some(Box::new(State::Init(Init {
            local_init: Some(()),
            remote_init: None,
        })));
        kex.init();
        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.local_init.is_some());
                    assert!(x.remote_init.is_none());
                }
                _ => assert!(false),
            },
        }
    }

    /// State shall be Init after remote init has been pushed
    #[test]
    fn test_client_kex_push_init_01() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c, vec![], vec![], vec![], vec![], vec![]);

        assert!(kex.push_init(ri).is_ok());

        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.local_init.is_none());
                    assert!(x.remote_init.is_some());
                }
                _ => assert!(false),
            },
        }
    }

    /// State shall contain both inits when remote init is pushed after local init
    #[test]
    fn test_client_kex_push_init_02() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c, vec![], vec![], vec![], vec![], vec![]);

        kex.state = Some(Box::new(State::Init(Init {
            local_init: Some(()),
            remote_init: None,
        })));

        assert!(kex.push_init(ri).is_ok());

        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.local_init.is_some());
                    assert!(x.remote_init.is_some());
                }
                _ => assert!(false),
            },
        }
    }

    /// Shall return ProtocolError when remote init is pushed twice
    #[test]
    fn test_client_kex_push_init_03() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c, vec![], vec![], vec![], vec![], vec![]);

        assert!(kex.push_init(ri.clone()).is_ok());
        match kex.push_init(ri) {
            Err(TransportError::ProtocolError) => (),
            _ => assert!(false),
        }
    }

    /// Shall return ProtocolError when remote init is pushed to incompatible state
    #[test]
    fn test_client_kex_push_init_04() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c, vec![], vec![], vec![], vec![], vec![]);
        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        kex.state = Some(Box::new(State::NewKeysSent(cc)));

        match kex.push_init(ri) {
            Err(TransportError::ProtocolError) => (),
            _ => assert!(false),
        }
    }

    /// Shall return ProtocolError when MSG_ECDH_INIT is pushed
    #[test]
    fn test_client_kex_push_ecdh_init() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let dh_secret = X25519::new();
        let dh_public = X25519::public(&dh_secret);
        let ecdh_init = MsgKexEcdhInit { dh_public };

        match kex.push_ecdh_init(ecdh_init) {
            Err(TransportError::ProtocolError) => (),
            _ => assert!(false),
        }
    }

    /// Shall go into HostKeyVerification state when MSG_ECDH_REPLY with valid signature is pushed
    #[test]
    fn test_client_kex_push_ecdh_reply_01() {
        use crate::algorithm::auth::*;
        use ed25519_dalek::Keypair;

        let keypair = Keypair::from_bytes(
            &[
                223, 172, 247, 249, 240, 155, 4, 236, 168, 114, 191, 70, 106, 161, 235, 150, 233,
                230, 152, 224, 83, 82, 245, 159, 35, 191, 255, 71, 23, 84, 237, 123, 217, 95, 204,
                68, 59, 27, 189, 73, 127, 200, 155, 100, 107, 123, 205, 53, 8, 124, 126, 128, 119,
                43, 108, 225, 133, 231, 36, 55, 164, 112, 190, 91,
            ][..],
        )
        .unwrap();

        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let local_id = Identification::default();
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(
            &config,
            verifier,
            remote_id.clone().into(),
            "hostname".into(),
        );

        let mut si: MsgKexInit = kex.local_init.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = keypair.public.to_bytes().clone();
        let host_key = Identity::PublicKey(PublicKey::Ed25519(SshEd25519PublicKey(host_key)));
        let client_dh_secret = X25519::new();
        let client_dh_public = X25519::public(&client_dh_secret);
        let server_dh_secret = X25519::new();
        let server_dh_public = X25519::public(&server_dh_secret);
        let k = X25519::diffie_hellman(server_dh_secret, &client_dh_public);

        // Prepare client state
        let algos = AlgorithmAgreement::agree(&kex.local_init, &si).unwrap();
        let state = State::Ecdh(Ecdh {
            remote_init: si.clone(),
            algos,
            dh_secret: client_dh_secret,
            sent: false,
        });
        kex.state = Some(Box::new(state));

        // Create "server" reply with correct signature (a bit complicated)
        let h: [u8; 32] = KexEcdhHash::<X25519> {
            client_identification: &local_id,
            server_identification: &remote_id,
            client_kex_init: &kex.local_init,
            server_kex_init: &si,
            server_host_key: &host_key,
            dh_client_key: &client_dh_public,
            dh_server_key: &server_dh_public,
            dh_secret: X25519::secret_as_ref(&k),
        }
        .sha256();
        let signature = SshEd25519Signature(keypair.sign(&h[..]).to_bytes());
        let signature = Signature::Ed25519(signature);
        let ecdh_reply = MsgKexEcdhReply {
            host_key,
            dh_public: server_dh_public,
            signature,
        };

        assert!(kex.push_ecdh_reply(ecdh_reply).is_ok());
        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::HostKeyVerification(_) => (),
                _ => assert!(false),
            },
        }
    }

    /// Shall return error when MSG_ECDH_REPLY with invalid signature is pushed
    #[test]
    fn test_client_kex_push_ecdh_reply_02() {
        use crate::algorithm::auth::*;

        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let mut si: MsgKexInit = kex.local_init.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = Identity::PublicKey(PublicKey::Ed25519(SshEd25519PublicKey([8; 32])));
        let client_dh_secret = X25519::new();
        let server_dh_secret = X25519::new();
        let server_dh_public = X25519::public(&server_dh_secret);

        // Prepare client state
        let algos = AlgorithmAgreement::agree(&kex.local_init, &si).unwrap();
        let state = State::Ecdh(Ecdh {
            remote_init: si.clone(),
            algos,
            dh_secret: client_dh_secret,
            sent: false,
        });
        kex.state = Some(Box::new(state));

        let ecdh_reply = MsgKexEcdhReply {
            host_key,
            dh_public: server_dh_public,
            signature: Signature::Ed25519(SshEd25519Signature([7; 64])),
        };

        match kex.push_ecdh_reply(ecdh_reply) {
            Err(TransportError::InvalidSignature) => (),
            _ => assert!(false),
        }
    }

    /// Shall return error when MSG_ECDH_REPLY is pushed onto incompatible state
    #[test]
    fn test_client_kex_push_ecdh_reply_03() {
        use crate::algorithm::auth::*;

        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let mut si: MsgKexInit = kex.local_init.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = Identity::PublicKey(PublicKey::Ed25519(SshEd25519PublicKey([8; 32])));
        let server_dh_secret = X25519::new();
        let server_dh_public = X25519::public(&server_dh_secret);

        let ecdh_reply = MsgKexEcdhReply {
            host_key,
            dh_public: server_dh_public,
            signature: Signature::Ed25519(SshEd25519Signature([7; 64])),
        };

        match kex.push_ecdh_reply(ecdh_reply) {
            Err(TransportError::ProtocolError) => (),
            _ => assert!(false),
        }
    }

    /// State shall be NewKeysReceived after MSG_NEWKEYS pushed
    #[test]
    fn test_client_kex_push_new_keys_01() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        kex.state = Some(Box::new(State::NewKeys((cc.clone(), cc))));

        assert!(kex.push_new_keys(0, 0).is_ok());

        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::NewKeysReceived(_) => (),
                _ => assert!(false),
            },
        }
    }

    /// State shall be None after MSG_NEWKEYS sent and received
    #[test]
    fn test_client_kex_push_new_keys_02() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        kex.state = Some(Box::new(State::NewKeysSent(cc)));

        assert!(kex.push_new_keys(0, 0).is_ok());
        assert!(kex.state.is_none());
    }

    /// Shall return ProtocolError when receiving MSG_NEWKEYS whlie kex is not in progress
    #[test]
    fn test_client_kex_push_new_keys_03() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        assert!(kex.push_new_keys(0, 0).is_err());
    }
}
*/
