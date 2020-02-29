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

    //FIXME
    fn is_sending_critical(&self) -> bool {
        match self.state {
            None => false,
            Some(ref x) => match x.as_ref() {
                State::Init(ref x) => x.client_init.is_some(),
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
                    let keys = KeyStreams::new_sha256(X25519::secret_as_ref(&k), &h, &self.session_id);
                    let enc = CipherConfig {
                        ea: ecdh.algos.ea_c2s,
                        ca: ecdh.algos.ca_c2s,
                        ma: ecdh.algos.ma_c2s,
                        ke: keys.c()
                    };
                    let dec = CipherConfig {
                        ea: ecdh.algos.ea_c2s,
                        ca: ecdh.algos.ca_c2s,
                        ma: ecdh.algos.ma_c2s,
                        ke: keys.d()
                    };
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
                State::NewKeysSent(dec) => Ok(dec),
                State::NewKeys((enc, dec)) => {
                    let state = State::NewKeysReceived(enc.clone());
                    self.state = Some(Box::new(state));
                    Ok(dec)
                }
                _ => Err(TransportError::ProtocolError),
            },
            _ => Err(TransportError::ProtocolError),
        }
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
                                    State::new_ecdh_x25519(algos, ci.clone(), si.clone())
                                }
                                Curve25519Sha256AtLibsshDotOrg::NAME => {
                                    State::new_ecdh_x25519(algos, ci.clone(), si.clone())
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

impl State {
    fn new_ecdh_x25519(
        algos: AlgorithmAgreement,
        client_init: MsgKexInit<&'static str>,
        server_init: MsgKexInit,
    ) -> Self {
        Self::Ecdh(Ecdh {
            algos,
            client_init,
            server_init,
            dh_secret: X25519::new(),
            sent: false,
        })
    }
}

struct Init {
    client_init: Option<MsgKexInit<&'static str>>,
    server_init: Option<MsgKexInit>,
}

struct Ecdh<A: EcdhAlgorithm> {
    algos: AlgorithmAgreement,
    client_init: MsgKexInit<&'static str>,
    server_init: MsgKexInit,
    dh_secret: A::EphemeralSecret,
    sent: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::*;

    #[test]
    fn test_kex_client_new() {
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
}
