use super::super::config::*;
use super::kex::*;
use crate::algorithm::kex::*;
use crate::algorithm::*;
use crate::util::*;

use std::time::Duration;

pub struct ClientKexMachine<V: HostKeyVerifier = Box<dyn HostKeyVerifier>> {
    state: ClientKexState,
    verifier: V,
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
    session_id: SessionId,
}

impl ClientKexMachine {
    fn reset(&mut self) {
        self.state = ClientKexState::Wait;
    }
}

pub enum ClientKexState {
    Wait,
    Init(Init),
    EcdhInit(Ecdh<X25519>),
    NewKeys((EncryptionConfig, DecryptionConfig)),
    NewKeysSent(DecryptionConfig),
    NewKeysReceived(EncryptionConfig),
}

impl ClientKexState {
    fn new_init(x: &ClientKexMachine, server_init: Option<MsgKexInit>) -> Self {
        Self::Init(Init {
            sent: false,
            client_init: MsgKexInit::new(
                KexCookie::random(),
                x.kex_algorithms.iter().map(|x| Into::into(*x)).collect(),
                x.host_key_algorithms
                    .iter()
                    .map(|x| Into::into(*x))
                    .collect(),
                x.encryption_algorithms
                    .iter()
                    .map(|x| Into::into(*x))
                    .collect(),
                x.mac_algorithms.iter().map(|x| Into::into(*x)).collect(),
                x.compression_algorithms
                    .iter()
                    .map(|x| Into::into(*x))
                    .collect(),
            ),
            server_init,
        })
    }
    // TODO: This needs to be extended in order to support other ECDH methods
    pub fn new_ecdh(
        client_init: MsgKexInit,
        server_init: MsgKexInit,
    ) -> Result<Self, TransportError> {
        if server_init
            .kex_algorithms
            .contains(&<Curve25519Sha256 as KexAlgorithm>::NAME.into())
        {
            return Ok(Self::EcdhInit(Ecdh {
                sent: false,
                client_init,
                server_init,
                dh_secret: X25519::new(),
            }));
        }
        return Err(TransportError::NoCommonKexAlgorithm);
    }
}

pub struct Init {
    sent: bool,
    client_init: MsgKexInit,
    server_init: Option<MsgKexInit>,
}

pub struct Ecdh<A: EcdhAlgorithm> {
    sent: bool,
    client_init: MsgKexInit,
    server_init: MsgKexInit,
    dh_secret: A::EphemeralSecret,
}

impl KexMachine for ClientKexMachine {
    fn new<C: TransportConfig>(config: &C, remote_id: Identification<String>) -> Self {
        let mut self_ = Self {
            state: ClientKexState::Wait,
            verifier: Box::new(IgnorantVerifier {}),
            local_id: config.identification().clone(),
            remote_id,
            interval_bytes: config.kex_interval_bytes(),
            interval_duration: config.kex_interval_duration(),
            next_kex_at_timeout: Delay::new(config.kex_interval_duration()),
            next_kex_at_bytes_sent: config.kex_interval_bytes(),
            next_kex_at_bytes_received: config.kex_interval_bytes(),
            kex_algorithms: intersection(config.kex_algorithms(), &SUPPORTED_KEX_ALGORITHMS[..]),
            mac_algorithms: intersection(config.mac_algorithms(), &SUPPORTED_MAC_ALGORITHMS[..]),
            host_key_algorithms: intersection(
                config.host_key_algorithms(),
                &SUPPORTED_HOST_KEY_ALGORITHMS[..],
            ),
            encryption_algorithms: intersection(
                config.encryption_algorithms(),
                &SUPPORTED_ENCRYPTION_ALGORITHMS[..],
            ),
            compression_algorithms: intersection(
                config.compression_algorithms(),
                &SUPPORTED_COMPRESSION_ALGORITHMS[..],
            ),
            session_id: SessionId::default(),
        };
        self_.init();
        self_
    }

    fn is_active(&self) -> bool {
        match self.state {
            ClientKexState::Wait => false,
            _ => true,
        }
    }

    //FIXME
    fn is_sending_critical(&self) -> bool {
        match self.state {
            ClientKexState::Wait => false,
            ClientKexState::Init(ref x) => x.sent,
            _ => true,
        }
    }

    //FIXME
    fn is_receiving_critical(&self) -> bool {
        match self.state {
            ClientKexState::Wait => false,
            ClientKexState::Init(ref x) => x.server_init.is_some(),
            _ => true,
        }
    }

    fn init(&mut self) {
        match self.state {
            ClientKexState::Wait => self.state = ClientKexState::new_init(self, None),
            _ => (),
        }
    }

    fn push_init(&mut self, server_init: MsgKexInit) -> Result<(), TransportError> {
        match &mut self.state {
            ClientKexState::Wait => {
                self.state = ClientKexState::new_init(self, Some(server_init));
            }
            ClientKexState::Init(init) => {
                if !init.sent {
                    init.server_init = Some(server_init);
                } else {
                    self.state = ClientKexState::new_ecdh(init.client_init.clone(), server_init)?;
                }
            }
            _ => return Err(TransportError::ProtocolError),
        }
        Ok(())
    }

    fn push_ecdh_init(&mut self, _: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        // Client shall not receive this message
        self.state = ClientKexState::Wait;
        Err(TransportError::ProtocolError)
    }

    fn push_ecdh_reply(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        match std::mem::replace(&mut self.state, ClientKexState::Wait) {
            ClientKexState::EcdhInit(mut ecdh) => {
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
                // Verify the host key
                assume(self.verifier.verify(&msg.host_key))
                    .ok_or(TransportError::HostKeyUnverifiable)?;
                // The session id is only computed during first kex and constant afterwards.
                self.session_id.update(h);
                let keys1 = KeyStreams::new_sha256(X25519::secret_as_ref(&k), &h, &self.session_id);
                let keys2 = KeyStreams::new_sha256(X25519::secret_as_ref(&k), &h, &self.session_id);
                let enc = CipherConfig::new_client_to_server(
                    &self.encryption_algorithms,
                    &self.compression_algorithms,
                    &self.mac_algorithms,
                    &ecdh.server_init,
                    keys1,
                )?;
                let dec = CipherConfig::new_server_to_client(
                    &self.encryption_algorithms,
                    &self.compression_algorithms,
                    &self.mac_algorithms,
                    &ecdh.server_init,
                    keys2,
                )?;
                self.state = ClientKexState::NewKeys((enc, dec));
                Ok(())
            }
            _ => Err(TransportError::ProtocolError),
        }
    }

    fn push_new_keys(&mut self) -> Result<CipherConfig, TransportError> {
        match std::mem::replace(&mut self.state, ClientKexState::Wait) {
            ClientKexState::NewKeys((enc_config, dec_config)) => {
                self.state = ClientKexState::NewKeysReceived(enc_config);
                Ok(dec_config)
            }
            ClientKexState::NewKeysSent(dec_config) => {
                self.reset();
                Ok(dec_config)
            }
            _ => Err(TransportError::ProtocolError),
        }
    }

    /// FIXME
    fn poll<F: FnMut(&mut Context, &KexOutput) -> Poll<Result<(), TransportError>>>(
        &mut self,
        cx: &mut Context,
        bytes_sent: u64,
        bytes_received: u64,
        mut f: F,
    ) -> Poll<Result<(), TransportError>> {
        loop {
            match &mut self.state {
                ClientKexState::Wait => (),
                ClientKexState::Init(x) => {
                    if !x.sent {
                        ready!(f(cx, &KexOutput::Init(x.client_init.clone())))?;
                        x.sent = true;
                        match &x.server_init {
                            None => (),
                            Some(server_init) => {
                                self.state = ClientKexState::new_ecdh(
                                    x.client_init.clone(),
                                    server_init.clone(),
                                )?;
                            }
                        }
                        continue;
                    }
                }
                ClientKexState::EcdhInit(x) => {
                    if !x.sent {
                        let msg: MsgKexEcdhInit<X25519> = MsgKexEcdhInit {
                            dh_public: X25519::public(&x.dh_secret),
                        };
                        ready!(f(cx, &KexOutput::EcdhInit(msg)))?;
                        x.sent = true;
                        continue;
                    }
                }
                ClientKexState::NewKeys((enc_config, dec_config)) => {
                    ready!(f(cx, &KexOutput::NewKeys(enc_config.clone())))?;
                    self.state = ClientKexState::NewKeysSent(dec_config.clone());
                    continue;
                }
                ClientKexState::NewKeysReceived(enc_config) => {
                    ready!(f(cx, &KexOutput::NewKeys(enc_config.clone())))?;
                    self.reset();
                    continue;
                }
                ClientKexState::NewKeysSent(_) => (),
            }
            return Poll::Ready(Ok(()));
        }
    }

    fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}
