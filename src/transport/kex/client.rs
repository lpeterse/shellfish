use super::super::config::*;
use super::kex::*;
use crate::algorithm::kex::*;
use crate::algorithm::*;

use std::time::Duration;

pub struct ClientKex {
    hostname: String,
    local_id: Identification<&'static str>,
    remote_id: Identification,
    interval_bytes: u64,
    interval_duration: Duration,
    next_kex_at_timeout: Delay,
    next_kex_at_bytes_sent: u64,
    next_kex_at_bytes_received: u64,
    kex_algorithms: Vec<&'static str>,
    mac_algorithms: Vec<&'static str>,
    host_key_algorithms: Vec<&'static str>,
    encryption_algorithms: Vec<&'static str>,
    compression_algorithms: Vec<&'static str>,
    state: Option<Box<State>>,
    verifier: Arc<Box<dyn HostKeyVerifier>>,
    session_id: SessionId,
}

impl ClientKex {
    pub fn new<C: TransportConfig>(
        config: &C,
        verifier: Arc<Box<dyn HostKeyVerifier>>,
        hostname: String,
        remote_id: Identification<String>,
    ) -> Self {
        let ka = intersection(config.kex_algorithms(), &KEX_ALGORITHMS[..]);
        let ma = intersection(config.mac_algorithms(), &MAC_ALGORITHMS[..]);
        let ha = intersection(config.host_key_algorithms(), &HOST_KEY_ALGORITHMS[..]);
        let ea = intersection(config.encryption_algorithms(), &ENCRYPTION_ALGORITHMS[..]);
        let ca = intersection(config.compression_algorithms(), &COMPRESSION_ALGORITHMS[..]);
        Self {
            hostname,
            local_id: config.identification().clone(),
            remote_id,
            interval_bytes: config.kex_interval_bytes(),
            interval_duration: config.kex_interval_duration(),
            next_kex_at_timeout: Delay::new(config.kex_interval_duration()),
            next_kex_at_bytes_sent: config.kex_interval_bytes(),
            next_kex_at_bytes_received: config.kex_interval_bytes(),
            kex_algorithms: ka,
            mac_algorithms: ma,
            host_key_algorithms: ha,
            encryption_algorithms: ea,
            compression_algorithms: ca,
            state: None,
            verifier,
            session_id: SessionId::default(),
        }
    }

    fn new_msg_kex_init(&self) -> MsgKexInit<&'static str> {
        let f = |s: &Vec<&'static str>| s.iter().map(|t| *t).collect();
        let ka = f(&self.kex_algorithms);
        let ha = f(&self.host_key_algorithms);
        let ea = f(&self.encryption_algorithms);
        let ma = f(&self.mac_algorithms);
        let ca = f(&self.compression_algorithms);
        MsgKexInit::new(KexCookie::random(), ka, ha, ea, ma, ca)
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
                State::Init(ref x) => x.client_init.is_some(),
                State::NewKeysSent(_) => false,
                _ => true,
            },
        }
    }

    fn is_receiving_critical(&self) -> bool {
        match self.state {
            None => false,
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => x.server_init.is_some(),
                State::NewKeysReceived(_) => false,
                _ => true,
            },
        }
    }

    fn init(&mut self) {
        match self.state {
            None => {
                let state = State::Init(Init {
                    client_init: None,
                    server_init: None,
                });
                self.state = Some(Box::new(state))
            }
            _ => (),
        }
    }

    fn push_init(&mut self, server_init: MsgKexInit) -> Result<(), TransportError> {
        match self.state {
            None => {
                let state = State::Init(Init {
                    client_init: None,
                    server_init: Some(server_init),
                });
                self.state = Some(Box::new(state));
                return Ok(());
            }
            Some(ref mut x) => match x.as_mut() {
                State::Init(ref mut i) if i.server_init.is_none() => {
                    i.server_init = Some(server_init);
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
                        client_kex_init: &ecdh.client_init,
                        server_kex_init: &ecdh.server_init,
                        server_host_key: &msg.host_key,
                        dh_client_key: &dh_public,
                        dh_server_key: &msg.dh_public,
                        dh_secret: k,
                    }
                    .sha256();
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
                    let fut = self.verifier.verify(&self.hostname, &msg.host_key);
                    self.state = Some(Box::new(State::HostKeyVerification((fut, enc, dec))));
                    return Ok(());
                }
                _ => (),
            },
            _ => (),
        }
        Err(TransportError::ProtocolError)
    }

    fn push_new_keys(&mut self) -> Result<CipherConfig, TransportError> {
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

    /// FIXME
    fn poll<F: FnMut(&mut Context, KexOutput) -> Poll<Result<(), TransportError>>>(
        &mut self,
        cx: &mut Context,
        bytes_sent: u64,
        bytes_received: u64,
        mut send: F,
    ) -> Poll<Result<(), TransportError>> {
        let msg_kex_init = self.new_msg_kex_init(); // FIXME
        loop {
            match self.state {
                Some(ref mut s) => match s.as_mut() {
                    State::Init(ref mut i) => {
                        if i.client_init.is_none() {
                            ready!(send(cx, KexOutput::Init(msg_kex_init.clone())))?;
                            i.client_init = Some(msg_kex_init.clone());
                        }
                        if let (Some(ci), Some(si)) =
                            (i.client_init.as_ref(), i.server_init.as_ref())
                        {
                            let algos = AlgorithmAgreement::agree(ci, &si)?;
                            let state = match algos.ka {
                                Curve25519Sha256::NAME => {
                                    State::Ecdh(Ecdh::new(ci.clone(), si.clone(), algos))
                                }
                                Curve25519Sha256AtLibsshDotOrg::NAME => {
                                    State::Ecdh(Ecdh::new(ci.clone(), si.clone(), algos))
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
                        ready!(verified.poll_unpin(cx))?;
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
    client_init: Option<MsgKexInit<&'static str>>,
    server_init: Option<MsgKexInit>,
}

struct Ecdh<A: EcdhAlgorithm> {
    client_init: MsgKexInit<&'static str>,
    server_init: MsgKexInit,
    algos: AlgorithmAgreement,
    dh_secret: A::EphemeralSecret,
    sent: bool,
}

impl Ecdh<X25519> {
    fn new(
        client_init: MsgKexInit<&'static str>,
        server_init: MsgKexInit,
        algos: AlgorithmAgreement,
    ) -> Self {
        Self {
            client_init,
            server_init,
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
        let hostname: String = "hostname".into();
        let remote_id: Identification = Identification::new("testing".into(), "".into());
        let kex = ClientKex::new(&config, verifier, hostname.clone(), remote_id.clone());

        assert_eq!(kex.hostname, hostname);
        assert_eq!(kex.local_id, *config.identification());
        assert_eq!(kex.remote_id, remote_id);
        assert_eq!(kex.interval_bytes, config.kex_interval_bytes);
        assert_eq!(kex.interval_duration, config.kex_interval_duration);
        assert_eq!(kex.next_kex_at_bytes_sent, config.kex_interval_bytes);
        assert_eq!(kex.next_kex_at_bytes_received, config.kex_interval_bytes);
        assert_eq!(kex.kex_algorithms, f(config.kex_algorithms));
        assert_eq!(kex.mac_algorithms, f(config.mac_algorithms));
        assert_eq!(kex.host_key_algorithms, f(config.host_key_algorithms));
        assert_eq!(kex.encryption_algorithms, f(config.encryption_algorithms));
        assert_eq!(kex.compression_algorithms, f(config.compression_algorithms));
        assert!(kex.state.is_none());
    }

    #[test]
    fn test_client_kex_is_active() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);
        assert!(!kex.is_active());
        kex.init();
        assert!(kex.is_active());
    }

    #[test]
    fn test_client_kex_is_sending_critical() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c.clone(), vec![], vec![], vec![], vec![], vec![]);
        let ci = MsgKexInit::<&'static str>::new(c, vec![], vec![], vec![], vec![], vec![]);
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
            client_init: None,
            server_init: None,
        })));
        assert!(!kex.is_sending_critical());
        // Shall be critical after MSG_KEX_INIT was sent
        kex.state = Some(Box::new(State::Init(Init {
            client_init: Some(ci),
            server_init: None,
        })));
        assert!(kex.is_sending_critical());
        // Shall not be critical after MSG_KEX_INIT was received
        kex.state = Some(Box::new(State::Init(Init {
            client_init: None,
            server_init: Some(ri),
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c.clone(), vec![], vec![], vec![], vec![], vec![]);
        let ci = MsgKexInit::<&'static str>::new(c, vec![], vec![], vec![], vec![], vec![]);
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
            client_init: None,
            server_init: None,
        })));
        assert!(!kex.is_receiving_critical());
        // Shall not be critical after MSG_KEX_INIT was sent
        kex.state = Some(Box::new(State::Init(Init {
            client_init: Some(ci),
            server_init: None,
        })));
        assert!(!kex.is_receiving_critical());
        // Shall be critical after MSG_KEX_INIT was received
        kex.state = Some(Box::new(State::Init(Init {
            client_init: None,
            server_init: Some(ri),
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        kex.init();
        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.client_init.is_none());
                    assert!(x.server_init.is_none());
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let c = KexCookie::random();
        let ci = MsgKexInit::<&'static str>::new(c, vec![], vec![], vec![], vec![], vec![]);

        kex.state = Some(Box::new(State::Init(Init {
            client_init: Some(ci),
            server_init: None,
        })));
        kex.init();
        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.client_init.is_some());
                    assert!(x.server_init.is_none());
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c, vec![], vec![], vec![], vec![], vec![]);

        assert!(kex.push_init(ri).is_ok());

        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.client_init.is_none());
                    assert!(x.server_init.is_some());
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let c = KexCookie::random();
        let ri = MsgKexInit::<String>::new(c.clone(), vec![], vec![], vec![], vec![], vec![]);
        let ci = MsgKexInit::<&'static str>::new(c, vec![], vec![], vec![], vec![], vec![]);

        kex.state = Some(Box::new(State::Init(Init {
            client_init: Some(ci),
            server_init: None,
        })));

        assert!(kex.push_init(ri).is_ok());

        match kex.state {
            None => assert!(false),
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => {
                    assert!(x.client_init.is_some());
                    assert!(x.server_init.is_some());
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id.clone());

        let mut ci = kex.new_msg_kex_init();
        ci.cookie = KexCookie([1; 16]);
        let mut si: MsgKexInit = ci.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = keypair.public.to_bytes().clone();
        let host_key = HostIdentity::Ed25519Key(SshEd25519PublicKey(host_key));
        let client_dh_secret = X25519::new();
        let client_dh_public = X25519::public(&client_dh_secret);
        let server_dh_secret = X25519::new();
        let server_dh_public = X25519::public(&server_dh_secret);
        let k = X25519::diffie_hellman(server_dh_secret, &client_dh_public);

        // Prepare client state
        let algos = AlgorithmAgreement::agree(&ci, &si).unwrap();
        let state = State::Ecdh(Ecdh {
            client_init: ci.clone(),
            server_init: si.clone(),
            algos,
            dh_secret: client_dh_secret,
            sent: false,
        });
        kex.state = Some(Box::new(state));

        // Create "server" reply with correct signature (a bit complicated)
        let h: [u8; 32] = KexEcdhHash::<X25519> {
            client_identification: &local_id,
            server_identification: &remote_id,
            client_kex_init: &ci,
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id.clone());

        let mut ci = kex.new_msg_kex_init();
        ci.cookie = KexCookie([1; 16]);
        let mut si: MsgKexInit = ci.clone().into();
        si.cookie = KexCookie([2; 16]);

        let host_key = HostIdentity::Ed25519Key(SshEd25519PublicKey([8; 32]));
        let client_dh_secret = X25519::new();
        let server_dh_secret = X25519::new();
        let server_dh_public = X25519::public(&server_dh_secret);

        // Prepare client state
        let algos = AlgorithmAgreement::agree(&ci, &si).unwrap();
        let state = State::Ecdh(Ecdh {
            client_init: ci.clone(),
            server_init: si.clone(),
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id.clone());

        let mut ci = kex.new_msg_kex_init();
        ci.cookie = KexCookie([1; 16]);
        let mut si: MsgKexInit = ci.clone().into();
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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        kex.state = Some(Box::new(State::NewKeys((cc.clone(), cc))));

        assert!(kex.push_new_keys().is_ok());

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
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        let ks = KeyStreams::new_sha256(&[][..], &[][..], &[][..]);
        let cc = CipherConfig {
            ea: "",
            ca: "",
            ma: None,
            ke: ks.c(),
        };

        kex.state = Some(Box::new(State::NewKeysSent(cc)));

        assert!(kex.push_new_keys().is_ok());
        assert!(kex.state.is_none());
    }

    /// Shall return ProtocolError when receiving MSG_NEWKEYS whlie kex is not in progress
    #[test]
    fn test_client_kex_push_new_keys_03() {
        let config = ClientConfig::default();
        let verifier: Arc<Box<dyn HostKeyVerifier>> = Arc::new(Box::new(AcceptingVerifier {}));
        let remote_id: Identification = Identification::new("foobar".into(), "".into());
        let mut kex = ClientKex::new(&config, verifier, "hostname".into(), remote_id);

        assert!(kex.push_new_keys().is_err());
    }
}
