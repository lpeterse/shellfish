use super::super::config::*;
use super::kex::*;
use crate::algorithm::kex::*;
use crate::algorithm::*;

use std::time::Duration;

/// The client side state machine for key exchange.
pub struct ClientKex {
    /// Host key verifier
    verifier: Arc<Box<dyn HostKeyVerifier>>,
    /// Mutable state (when kex in progress)
    state: Option<Box<State>>,
    /// Local identification string
    local_id: Identification<&'static str>,
    /// Local MSG_KEX_INIT (cookie shall be updated before re-exchange)
    local_init: MsgKexInit<&'static str>,
    /// Remote identification string
    remote_id: Identification,
    /// Remote name (hostname) for host key verification
    remote_name: String,
    /// Rekeying interval (bytes sent or received)
    interval_bytes: u64,
    /// Rekeying interval (time passed since last kex)
    interval_duration: Duration,
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
    pub fn new<C: TransportConfig>(
        config: &C,
        verifier: Arc<Box<dyn HostKeyVerifier>>,
        remote_id: Identification<String>,
        remote_name: String,
    ) -> Self {
        let ka = intersection(config.kex_algorithms(), &KEX_ALGORITHMS[..]);
        let ma = intersection(config.mac_algorithms(), &MAC_ALGORITHMS[..]);
        let ha = intersection(config.host_key_algorithms(), &HOST_KEY_ALGORITHMS[..]);
        let ea = intersection(config.encryption_algorithms(), &ENCRYPTION_ALGORITHMS[..]);
        let ca = intersection(config.compression_algorithms(), &COMPRESSION_ALGORITHMS[..]);
        Self {
            verifier,
            state: None,
            local_id: config.identification().clone(),
            local_init: MsgKexInit::new(KexCookie::random(), ka, ha, ea, ma, ca),
            remote_id,
            remote_name,
            interval_bytes: config.kex_interval_bytes(),
            interval_duration: config.kex_interval_duration(),
            next_at: Delay::new(config.kex_interval_duration()),
            next_at_bytes_sent: config.kex_interval_bytes(),
            next_at_bytes_received: config.kex_interval_bytes(),
            session_id: SessionId::default(),
        }
    }
}

impl Kex for ClientKex {
    fn is_active(&self) -> bool {
        self.state.is_some()
    }

    fn is_sending_critical(&self) -> bool {
        match self.state {
            None => false,
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => x.local_init.is_some(),
                State::NewKeysSent(_) => false,
                _ => true,
            },
        }
    }

    fn is_receiving_critical(&self) -> bool {
        match self.state {
            None => false,
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => x.remote_init.is_some(),
                State::NewKeysReceived(_) => false,
                _ => true,
            },
        }
    }

    fn init(&mut self) {
        match self.state {
            None => {
                let state = State::Init(Init {
                    local_init: None,
                    remote_init: None,
                });
                self.state = Some(Box::new(state))
            }
            _ => (),
        }
    }

    fn push_init(&mut self, remote_init: MsgKexInit) -> Result<(), TransportError> {
        match self.state {
            None => {
                let state = State::Init(Init {
                    local_init: None,
                    remote_init: Some(remote_init),
                });
                self.state = Some(Box::new(state));
                return Ok(());
            }
            Some(ref mut x) => match x.as_mut() {
                State::Init(ref mut i) if i.remote_init.is_none() => {
                    i.remote_init = Some(remote_init);
                    return Ok(());
                }
                _ => (),
            },
        }
        Err(TransportError::ProtocolError)?
    }

    fn push_ecdh_init(&mut self, _: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        Err(TransportError::ProtocolError)
    }

    fn push_ecdh_reply(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        match std::mem::replace(&mut self.state, None) {
            Some(x) => match *x {
                State::Ecdh(mut ecdh) => {
                    // Compute the DH shared secret (create a new placeholder while
                    // the actual secret get consumed in the operation).
                    let dh_secret = std::mem::replace(&mut ecdh.dh_secret, X25519::new());
                    let dh_public = X25519::public(&dh_secret);
                    let k = X25519::diffie_hellman(dh_secret, &msg.dh_public);
                    let k = X25519::secret_as_ref(&k);
                    // Compute the exchange hash over the data exchanged so far.
                    let h: [u8; 32] = KexEcdhHash::<X25519> {
                        client_identification: &self.local_id,
                        server_identification: &self.remote_id,
                        client_kex_init: &self.local_init,
                        server_kex_init: &ecdh.remote_init,
                        server_host_key: &msg.host_key,
                        dh_client_key: &dh_public,
                        dh_server_key: &msg.dh_public,
                        dh_secret: k,
                    }
                    .sha256();
                    log::error!("{:?} {:?} {:?}", k, dh_public, msg.dh_public);
                    // Verify the host key signature
                    msg.signature.verify(&msg.host_key, &h[..])?;
                    // The session id is only computed during first kex and constant afterwards
                    self.session_id.update(h);
                    let keys = KeyStreams::new_sha256(k, &h, &self.session_id);
                    let enc = CipherConfig {
                        ea: ecdh.algos.ea_c2s,
                        ca: ecdh.algos.ca_c2s,
                        ma: ecdh.algos.ma_c2s,
                        ke: keys.c(),
                    };
                    let dec = CipherConfig {
                        ea: ecdh.algos.ea_s2c,
                        ca: ecdh.algos.ca_s2c,
                        ma: ecdh.algos.ma_s2c,
                        ke: keys.d(),
                    };
                    let fut = self.verifier.verify(&self.remote_name, &msg.host_key);
                    self.state = Some(Box::new(State::HostKeyVerification((fut, enc, dec))));
                    return Ok(());
                }
                _ => (),
            },
            _ => (),
        }
        Err(TransportError::ProtocolError)
    }

    fn push_new_keys(
        &mut self,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Result<CipherConfig, TransportError> {
        self.next_at.reset(self.interval_duration);
        self.next_at_bytes_sent = bytes_sent + self.interval_bytes;
        self.next_at_bytes_received = bytes_received + self.interval_bytes;

        match std::mem::replace(&mut self.state, None) {
            Some(x) => match *x {
                State::NewKeysSent(dec) => return Ok(dec),
                State::NewKeys((enc, dec)) => {
                    let state = State::NewKeysReceived(enc.clone());
                    self.state = Some(Box::new(state));
                    return Ok(dec);
                }
                _ => (),
            },
            _ => (),
        }
        Err(TransportError::ProtocolError)
    }

    fn poll<F: FnMut(&mut Context, KexOutput) -> Poll<Result<(), TransportError>>>(
        &mut self,
        cx: &mut Context,
        bytes_sent: u64,
        bytes_received: u64,
        mut send: F,
    ) -> Poll<Result<(), TransportError>> {
        // Determine whether kex is required according to timeout or traffic.
        // This might evaluate to true even if kex is already in progress, but is harmless.
        let a = self.next_at.poll_unpin(cx).is_ready();
        let b = self.next_at_bytes_sent <= bytes_sent;
        let c = self.next_at_bytes_received <= bytes_received;
        if a || b || c {
            self.init();
        }
        // Pop all pending kex messages from the state machine. Return Ready if no more messages
        // available or Pending if sending such a message failed.
        loop {
            match self.state {
                Some(ref mut s) => match s.as_mut() {
                    State::Init(ref mut i) => {
                        if i.local_init.is_none() {
                            self.local_init.cookie = KexCookie::random();
                            ready!(send(cx, KexOutput::Init(self.local_init.clone())))?;
                            i.local_init = Some(());
                        }
                        if let Some(si) = i.remote_init.as_ref() {
                            let algos = AlgorithmAgreement::agree(&self.local_init, &si)?;
                            let state = match algos.ka {
                                Curve25519Sha256::NAME => State::Ecdh(Ecdh::new(si.clone(), algos)),
                                Curve25519Sha256AtLibsshDotOrg::NAME => {
                                    State::Ecdh(Ecdh::new(si.clone(), algos))
                                }
                                _ => Err(TransportError::NoCommonKexAlgorithm)?,
                            };
                            self.state = Some(Box::new(state));
                            continue;
                        }
                    }
                    State::Ecdh(ref mut i) => {
                        if !i.sent {
                            let msg: MsgKexEcdhInit<X25519> = MsgKexEcdhInit {
                                dh_public: X25519::public(&i.dh_secret),
                            };
                            ready!(send(cx, KexOutput::EcdhInit(msg)))?;
                            i.sent = true;
                            continue;
                        }
                    }
                    State::HostKeyVerification((verified, enc, dec)) => {
                        ready!(core::pin::Pin::as_mut(verified).poll(cx))?;
                        self.state = Some(Box::new(State::NewKeys((enc.clone(), dec.clone()))));
                        continue;
                    }
                    State::NewKeys((enc, dec)) => {
                        ready!(send(cx, KexOutput::NewKeys(enc.clone())))?;
                        self.state = Some(Box::new(State::NewKeysSent(dec.clone())));
                        continue;
                    }
                    State::NewKeysReceived(cipher) => {
                        ready!(send(cx, KexOutput::NewKeys(cipher.clone())))?;
                        self.state = None;
                        continue;
                    }
                    State::NewKeysSent(_) => (),
                },
                None => (),
            }
            return Poll::Ready(Ok(()));
        }
    }

    fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

enum State {
    Init(Init),
    Ecdh(Ecdh<X25519>),
    HostKeyVerification((VerificationFuture, EncryptionConfig, DecryptionConfig)),
    NewKeys((EncryptionConfig, DecryptionConfig)),
    NewKeysSent(DecryptionConfig),
    NewKeysReceived(EncryptionConfig),
}

struct Init {
    local_init: Option<()>,
    remote_init: Option<MsgKexInit>,
}

struct Ecdh<A: EcdhAlgorithm> {
    remote_init: MsgKexInit,
    algos: AlgorithmAgreement,
    dh_secret: A::EphemeralSecret,
    sent: bool,
}

impl Ecdh<X25519> {
    fn new(remote_init: MsgKexInit, algos: AlgorithmAgreement) -> Self {
        Self {
            remote_init,
            algos,
            dh_secret: X25519::new(),
            sent: false,
        }
    }
}

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
        use crate::algorithm::authentication::*;
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
        let host_key = HostIdentity::Ed25519Key(SshEd25519PublicKey(host_key));
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
        let signature = HostSignature::Ed25519Signature(signature);
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
        use crate::algorithm::authentication::*;

        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let mut si: MsgKexInit = kex.local_init.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = HostIdentity::Ed25519Key(SshEd25519PublicKey([8; 32]));
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
            signature: HostSignature::Ed25519Signature(SshEd25519Signature([7; 64])),
        };

        match kex.push_ecdh_reply(ecdh_reply) {
            Err(TransportError::InvalidSignature) => (),
            _ => assert!(false),
        }
    }

    /// Shall return error when MSG_ECDH_REPLY is pushed onto incompatible state
    #[test]
    fn test_client_kex_push_ecdh_reply_03() {
        use crate::algorithm::authentication::*;

        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, remote_id, "hostname".into());

        let mut si: MsgKexInit = kex.local_init.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = HostIdentity::Ed25519Key(SshEd25519PublicKey([8; 32]));
        let server_dh_secret = X25519::new();
        let server_dh_public = X25519::public(&server_dh_secret);

        let ecdh_reply = MsgKexEcdhReply {
            host_key,
            dh_public: server_dh_public,
            signature: HostSignature::Ed25519Signature(SshEd25519Signature([7; 64])),
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
