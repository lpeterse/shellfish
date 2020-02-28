use super::super::config::*;
use super::kex::*;
use crate::algorithm::kex::*;
use crate::algorithm::*;

use std::time::Duration;

pub struct ClientKex {
    hostname: String,
    local_id: Identification<&'static str>,
    remote_id: Identification<String>,
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
        let mut self_ = Self {
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
        };
        self_.init();
        self_
    }
}

impl Kex for ClientKex {
    fn is_active(&self) -> bool {
        self.state.is_some()
    }

    //FIXME
    fn is_sending_critical(&self) -> bool {
        match self.state {
            None => false,
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => x.sent,
                _ => true,
            },
        }
    }

    //FIXME
    fn is_receiving_critical(&self) -> bool {
        match self.state {
            None => false,
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => x.server_init.is_some(),
                _ => true,
            },
        }
    }

    fn init(&mut self) {
        match self.state {
            None => self.state = Some(Box::new(State::new_init(self, None))),
            _ => (),
        }
    }

    fn push_init(&mut self, server_init: MsgKexInit) -> Result<(), TransportError> {
        match self.state {
            None => {
                let state = State::new_init(self, Some(server_init));
                self.state = Some(Box::new(state));
            }
            Some(ref mut x) => match x.as_mut() {
                State::Init(ref mut i) => {
                    if !i.sent {
                        i.server_init = Some(server_init);
                    } else {
                        let state = State::new_ecdh(i.client_init.clone(), server_init)?;
                        self.state = Some(Box::new(state));
                    }
                }
                _ => Err(TransportError::ProtocolError)?,
            },
        }
        Ok(())
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
                    // Compute the exchange hash over the data exchanged so far.
                    let h: [u8; 32] = KexEcdhHash::<X25519> {
                        client_identification: &self.local_id,
                        server_identification: &self.remote_id,
                        client_kex_init: &ecdh.client_init,
                        server_kex_init: &ecdh.server_init,
                        server_host_key: &msg.host_key,
                        dh_client_key: &dh_public,
                        dh_server_key: &msg.dh_public,
                        dh_secret: X25519::secret_as_ref(&k),
                    }
                    .sha256();
                    // Verify the host key signature
                    msg.signature.verify(&msg.host_key, &h[..])?;
                    // The session id is only computed during first kex and constant afterwards
                    self.session_id.update(h);
                    let enc = CipherConfig::new_client_to_server(
                        &self.encryption_algorithms,
                        &self.compression_algorithms,
                        &self.mac_algorithms,
                        &ecdh.server_init,
                        KeyStreams::new_sha256(X25519::secret_as_ref(&k), &h, &self.session_id),
                    )?;
                    let dec = CipherConfig::new_server_to_client(
                        &self.encryption_algorithms,
                        &self.compression_algorithms,
                        &self.mac_algorithms,
                        &ecdh.server_init,
                        KeyStreams::new_sha256(X25519::secret_as_ref(&k), &h, &self.session_id),
                    )?;
                    let verified = self.verifier.verify(&self.hostname, &msg.host_key);
                    self.state = Some(Box::new(State::HostKeyVerification((verified, enc, dec))));
                }
                _ => Err(TransportError::ProtocolError)?,
            },
            _ => Err(TransportError::ProtocolError)?,
        }
        Ok(())
    }

    fn push_new_keys(&mut self) -> Result<CipherConfig, TransportError> {
        match std::mem::replace(&mut self.state, None) {
            Some(x) => match *x {
                State::NewKeysSent(dec_config) => Ok(dec_config),
                State::NewKeys((enc_config, dec_config)) => {
                    let state = State::NewKeysReceived(enc_config);
                    self.state = Some(Box::new(state));
                    Ok(dec_config)
                }
                _ => Err(TransportError::ProtocolError),
            },
            _ => Err(TransportError::ProtocolError),
        }
    }

    /// FIXME
    fn poll<F: FnMut(&mut Context, &KexOutput) -> Poll<Result<(), TransportError>>>(
        &mut self,
        cx: &mut Context,
        bytes_sent: u64,
        bytes_received: u64,
        mut send: F,
    ) -> Poll<Result<(), TransportError>> {
        loop {
            match self.state {
                Some(ref mut s) => match s.as_mut() {
                    State::Init(ref mut i) => {
                        if !i.sent {
                            ready!(send(cx, &KexOutput::Init(i.client_init.clone())))?;
                            i.sent = true;
                            match i.server_init {
                                None => (),
                                Some(ref server_init) => {
                                    let si = server_init.clone();
                                    let ci = i.client_init.clone();
                                    let state = State::new_ecdh(ci, si)?;
                                    self.state = Some(Box::new(state));
                                }
                            }
                            continue;
                        }
                    }
                    State::Ecdh(ref mut i) => {
                        if !i.sent {
                            let msg: MsgKexEcdhInit<X25519> = MsgKexEcdhInit {
                                dh_public: X25519::public(&i.dh_secret),
                            };
                            ready!(send(cx, &KexOutput::EcdhInit(msg)))?;
                            i.sent = true;
                            continue;
                        }
                    }
                    State::HostKeyVerification((verified, enc_config, dec_config)) => {
                        ready!(verified.poll_unpin(cx))?;
                        self.state = Some(Box::new(State::NewKeys((
                            enc_config.clone(),
                            dec_config.clone(),
                        ))));
                        continue;
                    }
                    State::NewKeys((enc_config, dec_config)) => {
                        ready!(send(cx, &KexOutput::NewKeys(enc_config.clone())))?;
                        self.state = Some(Box::new(State::NewKeysSent(dec_config.clone())));
                        continue;
                    }
                    State::NewKeysReceived(enc_config) => {
                        ready!(send(cx, &KexOutput::NewKeys(enc_config.clone())))?;
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

impl State {
    fn new_init(x: &ClientKex, server_init: Option<MsgKexInit>) -> Self {
        let f = |s: &Vec<&str>| s.iter().map(|t| String::from(*t)).collect();
        let ka = f(&x.kex_algorithms);
        let ha = f(&x.host_key_algorithms);
        let ea = f(&x.encryption_algorithms);
        let ma = f(&x.mac_algorithms);
        let ca = f(&x.compression_algorithms);
        Self::Init(Init {
            sent: false,
            client_init: MsgKexInit::new(KexCookie::random(), ka, ha, ea, ma, ca),
            server_init,
        })
    }

    fn new_ecdh(client_init: MsgKexInit, server_init: MsgKexInit) -> Result<Self, TransportError> {
        let ecdh = &<Curve25519Sha256 as KexAlgorithm>::NAME.into();
        if server_init.kex_algorithms.contains(ecdh) {
            Ok(Self::Ecdh(Ecdh {
                sent: false,
                client_init,
                server_init,
                dh_secret: X25519::new(),
            }))
        } else {
            Err(TransportError::NoCommonKexAlgorithm)
        }
    }
}

struct Init {
    sent: bool,
    client_init: MsgKexInit,
    server_init: Option<MsgKexInit>,
}

struct Ecdh<A: EcdhAlgorithm> {
    sent: bool,
    client_init: MsgKexInit,
    server_init: MsgKexInit,
    dh_secret: A::EphemeralSecret,
}
