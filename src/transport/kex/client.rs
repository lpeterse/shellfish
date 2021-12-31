use super::super::*;
use crate::transport::keys::KeyAlgorithm;
use crate::util::BoxFuture;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// The client side state machine for key exchange.
pub struct ClientKex {
    config: Arc<TransportConfig>,
    /// Host identity verifier
    host_verifier: Arc<dyn HostVerifier>,
    /// Remote hostname for host key verification
    host_name: String,
    /// Remote port
    host_port: u16,
    /// Remote identification string
    host_id: Identification,
    /// Host identity verification task
    verify: Option<BoxFuture<Result<(), HostVerificationError>>>,
    /// Session id (only after after initial kex, constant afterwards)
    session_id: Option<Secret>,
    /// Mutable state (when kex in progress)
    state: State,
    /// Output buffer
    output: VecDeque<KexMessage>,
}

impl ClientKex {
    pub fn new(
        config: &Arc<TransportConfig>,
        host_verifier: &Arc<dyn HostVerifier>,
        host_name: &str,
        host_port: u16,
        host_id: Identification<String>,
    ) -> Self {
        let mut self_ = Self {
            config: config.clone(),
            host_verifier: host_verifier.clone(),
            host_name: host_name.into(),
            host_port,
            host_id,
            verify: None,
            session_id: None,
            state: State::Idle,
            output: VecDeque::new(),
        };
        self_.init();
        self_
    }
}

impl Kex for ClientKex {
    fn init(&mut self) {
        match &self.state {
            State::Idle => {
                let cnf = &self.config;
                let cki = KexCookie::random();
                let msg = Arc::new(MsgKexInit::new_from_config(cki, cnf));
                self.output.push_back(KexMessage::Init(msg.clone()));
                self.state = State::Init(Box::new(StateInit { init: msg }));
            }
            _ => (),
        }
    }

    fn push_init(&mut self, msg: MsgKexInit) -> Result<(), TransportError> {
        self.init();
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Init(x) => {
                let init_client = x.init;
                let init_server = msg;
                let client_ka = &init_client.kex_algorithms;
                let server_ka = &init_server.kex_algorithms;
                let common_ka = common(client_ka, server_ka);
                let common_ka = common_ka.ok_or(TransportError::NoCommonKexAlgorithm)?;
                match common_ka {
                    Curve25519Sha256::NAME => {
                        let ecdh_secret = X25519::secret_new();
                        let ecdh_public = X25519::public_from_secret(&ecdh_secret);
                        let ecdh_public = X25519::public_as_bytes(&ecdh_public).into();
                        let ecdh_client = MsgKexEcdhInit::new(ecdh_public);
                        let ecdh_client = Arc::new(ecdh_client);
                        let s = StateEcdhCurve25519Sha256 {
                            init_client,
                            init_server,
                            ecdh_secret,
                        };
                        self.output.push_back(KexMessage::EcdhInit(ecdh_client));
                        self.state = State::EcdhCurve25519Sha256(Box::new(s));
                        Ok(())
                    }
                    _ => Err(TransportError::NoCommonKexAlgorithm),
                }
            }
            _ => Err(TransportError::InvalidState),
        }
    }

    fn push_ecdh_reply(&mut self, msg: MsgKexEcdhReply) -> Result<(), TransportError> {
        match std::mem::replace(&mut self.state, State::Idle) {
            State::EcdhCurve25519Sha256(x) => {
                // Compute the DH shared secret (create a new placeholder while
                // the actual secret gets consumed in the operation).
                const EINSIG: TransportError = TransportError::InvalidSignature;
                let dh_public_client = X25519::public_from_secret(&x.ecdh_secret);
                let dh_public_server = X25519::public_from_bytes(&msg.dh_public).ok_or(EINSIG)?;
                let k = X25519::diffie_hellman(x.ecdh_secret, &dh_public_server);
                // Compute the exchange hash over the data exchanged so far.
                let h: Secret = KexEcdhHash::<_, _> {
                    client_id: &self.config.identification,
                    server_id: &self.host_id,
                    client_kex_init: &x.init_client,
                    server_kex_init: &x.init_server,
                    server_host_key: &msg.host_key,
                    dh_client_key: dh_public_client.as_bytes(),
                    dh_server_key: dh_public_server.as_bytes(),
                    dh_secret: &k,
                }
                .sha256();
                // Verify the host key signature
                msg.signature.verify(&msg.host_key, h.as_ref())?;
                // The session id is only computed during first kex and constant afterwards
                let sid = self.session_id.get_or_insert_with(|| h.clone());
                let alg = KeyAlgorithm::Sha256;
                let kis = &x.init_server;
                let kic = &x.init_client;
                let (c2s, s2c) = ciphers(common, alg, kis, kic, &k, &h, &sid)?;
                let hn = &self.host_name;
                let hp = self.host_port;
                let hk = &msg.host_key;
                self.verify = Some(self.host_verifier.verify(hn, hp, hk));
                self.output.push_back(KexMessage::NewKeys(Box::new(c2s)));
                self.state = State::NewKeys(s2c);
                Ok(())
            }
            _ => Err(TransportError::InvalidState),
        }
    }

    fn push_new_keys(&mut self) -> Result<Box<CipherConfig>, TransportError> {
        match std::mem::replace(&mut self.state, State::Idle) {
            State::NewKeys(cipher_config) => Ok(Box::new(cipher_config)),
            _ => Err(TransportError::InvalidState),
        }
    }

    fn session_id(&self) -> Option<&Secret> {
        self.session_id.as_ref()
    }

    fn poll(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<&mut VecDeque<KexMessage>, TransportError>> {
        // Poll the host key verification if it is in progress
        if let Some(fut) = &mut self.verify {
            ready!(Pin::new(fut).poll(cx)).map_err(TransportError::InvalidIdentity)?;
            self.verify = None;
        }
        // Return a reference on the output messages queue
        Poll::Ready(Ok(&mut self.output))
    }
}

impl std::fmt::Debug for ClientKex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientKex {{ ... }}")
    }
}

enum State {
    Idle,
    Init(Box<StateInit>),
    EcdhCurve25519Sha256(Box<StateEcdhCurve25519Sha256>),
    NewKeys(CipherConfig),
}

struct StateInit {
    init: Arc<MsgKexInit<&'static str>>,
}

struct StateEcdhCurve25519Sha256 {
    init_client: Arc<MsgKexInit<&'static str>>,
    init_server: MsgKexInit<String>,
    ecdh_secret: <X25519 as EcdhAlgorithm>::EphemeralSecret,
}
