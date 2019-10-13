use super::super::config::*;
use super::kex::*;
use crate::algorithm::kex::*;
use crate::algorithm::*;

pub struct ClientKexMachine {
    pub state: ClientKexState,
    pub interval_bytes: u64,
    pub interval_duration: std::time::Duration,
    pub next_kex_at_bytes_sent: u64,
    pub next_kex_at_bytes_received: u64,
    pub kex_algorithms: Vec<&'static str>,
    pub mac_algorithms: Vec<&'static str>,
    pub host_key_algorithms: Vec<&'static str>,
    pub encryption_algorithms: Vec<&'static str>,
    pub compression_algorithms: Vec<&'static str>,
    pub session_id: Option<SessionId>,
}

pub enum ClientKexState {
    Delay(Delay),
    Init(Init),
    Ecdh(Ecdh<X25519>),
    NewKeys(NewKeys),
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
    pub fn new_ecdh(client_init: MsgKexInit, server_init: MsgKexInit) -> Result<Self, KexError> {
        if server_init
            .kex_algorithms
            .contains(&<Curve25519Sha256 as KexAlgorithm>::NAME.into())
        {
            return Ok(Self::Ecdh(Ecdh {
                sent: false,
                client_init,
                server_init,
                dh_secret: X25519::new(),
            }));
        }
        return Err(KexError::NoCommonKexAlgorithm);
    }
}

pub struct Init {
    pub sent: bool,
    pub client_init: MsgKexInit,
    pub server_init: Option<MsgKexInit>,
}

pub struct Ecdh<A: EcdhAlgorithm> {
    pub sent: bool,
    pub client_init: MsgKexInit,
    pub server_init: MsgKexInit,
    pub dh_secret: A::EphemeralSecret,
}

pub struct NewKeys {
    pub sent: bool,
    pub client_init: MsgKexInit,
    pub server_init: MsgKexInit,
    pub key_streams: KeyStreams,
}

impl KexMachine for ClientKexMachine {
    fn new<C: TransportConfig>(config: &C) -> Self {
        let mut self_ = Self {
            state: ClientKexState::Delay(Delay::new(Default::default())),
            interval_bytes: config.kex_interval_bytes(),
            interval_duration: config.kex_interval_duration(),
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
            session_id: None,
        };
        self_.init_local();
        self_
    }

    fn is_init_sent(&self) -> bool {
        match self.state {
            ClientKexState::Delay(_) => false,
            ClientKexState::Init(ref x) => x.sent,
            _ => true,
        }
    }

    fn is_init_received(&self) -> bool {
        match self.state {
            ClientKexState::Delay(_) => false,
            ClientKexState::Init(ref x) => x.server_init.is_some(),
            _ => true,
        }
    }

    fn is_in_progress<T: Socket>(
        &mut self,
        cx: &mut Context,
        t: &mut Transmitter<T>,
    ) -> Result<bool, KexError> {
        match &mut self.state {
            ClientKexState::Delay(timer) => match timer.poll_unpin(cx) {
                Poll::Pending => {
                    if t.bytes_sent() >= self.next_kex_at_bytes_sent
                        || t.bytes_received() >= self.next_kex_at_bytes_received
                    {
                        self.init_local();
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Poll::Ready(Ok(())) => {
                    self.init_local();
                    Ok(true)
                }
                Poll::Ready(Err(e)) => Err(e.into()),
            },
            _ => Ok(true),
        }
    }

    fn init_local(&mut self) {
        match self.state {
            ClientKexState::Delay(_) => self.state = ClientKexState::new_init(self, None),
            _ => (),
        }
    }

    fn init_remote(&mut self, server_init: MsgKexInit) -> Result<(), KexError> {
        match &mut self.state {
            ClientKexState::Delay(_) => {
                self.state = ClientKexState::new_init(self, Some(server_init));
            }
            ClientKexState::Init(init) => {
                if !init.sent {
                    init.server_init = Some(server_init);
                } else {
                    self.state = ClientKexState::new_ecdh(init.client_init.clone(), server_init)?;
                }
            }
            _ => return Err(KexError::ProtocolError),
        }
        Ok(())
    }

    fn consume<T: Socket>(&mut self, t: &mut Transmitter<T>) -> Result<(), KexError> {
        match t.decode() {
            Some(msg) => {
                log::debug!("Received MSG_ECDH_REPLY");
                match &mut self.state {
                    ClientKexState::Ecdh(ecdh) => {
                        let reply: MsgKexEcdhReply<X25519> = msg;
                        // Compute the DH shared secret (create a new placeholder while
                        // the actual secret get consumed in the operation).
                        let dh_secret = std::mem::replace(&mut ecdh.dh_secret, X25519::new());
                        let dh_public = X25519::public(&dh_secret);
                        let k = X25519::diffie_hellman(dh_secret, &reply.dh_public);
                        // Compute the exchange hash over the data exchanged so far.
                        let h: [u8; 32] = KexEcdhHash::<X25519> {
                            client_identification: &t.local_id(),
                            server_identification: &t.remote_id(),
                            client_kex_init: &ecdh.client_init,
                            server_kex_init: &ecdh.server_init,
                            server_host_key: &reply.host_key,
                            dh_client_key: &dh_public,
                            dh_server_key: &reply.dh_public,
                            dh_secret: X25519::secret_as_ref(&k),
                        }
                        .sha256();
                        // The session id is only computed during first kex and constant afterwards.
                        self.state = ClientKexState::NewKeys(NewKeys {
                            sent: false,
                            client_init: ecdh.client_init.clone(),
                            server_init: ecdh.server_init.clone(),
                            key_streams: KeyStreams::new_sha256(
                                X25519::secret_as_ref(&k),
                                &h,
                                self.session_id.get_or_insert_with(|| SessionId::new(h)),
                            ),
                        });

                        t.consume();
                        return Ok(());
                    }
                    _ => (),
                }
            }
            None => (),
        }
        match t.decode() {
            Some(msg) => {
                let _: MsgNewKeys = msg;
                log::debug!("Received MSG_NEW_KEYS");
                let state = ClientKexState::Delay(Delay::new(self.interval_duration));
                let state = std::mem::replace(&mut self.state, state);
                match state {
                    ClientKexState::NewKeys(mut x) => {
                        let encryption_algorithm_client_to_server = common(
                            &self.encryption_algorithms,
                            &x.server_init.encryption_algorithms_client_to_server,
                        )
                        .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                        let encryption_algorithm_server_to_client = common(
                            &self.encryption_algorithms,
                            &x.server_init.encryption_algorithms_server_to_client,
                        )
                        .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                        let compression_algorithm_client_to_server = common(
                            &self.compression_algorithms,
                            &x.server_init.compression_algorithms_client_to_server,
                        )
                        .ok_or(KexError::NoCommonCompressionAlgorithm)?;
                        let compression_algorithm_server_to_client = common(
                            &self.compression_algorithms,
                            &x.server_init.compression_algorithms_server_to_client,
                        )
                        .ok_or(KexError::NoCommonCompressionAlgorithm)?;
                        let mac_algorithm_client_to_server = common(
                            &self.mac_algorithms,
                            &x.server_init.mac_algorithms_client_to_server,
                        );
                        let mac_algorithm_server_to_client = common(
                            &self.mac_algorithms,
                            &x.server_init.mac_algorithms_server_to_client,
                        );
                        t.encryption_ctx().update(
                            &encryption_algorithm_client_to_server,
                            &compression_algorithm_client_to_server,
                            mac_algorithm_client_to_server,
                            &mut x.key_streams.c(),
                        ).ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                        t.decryption_ctx().update(
                            &encryption_algorithm_server_to_client,
                            &compression_algorithm_server_to_client,
                            mac_algorithm_server_to_client,
                            &mut x.key_streams.d(),
                        ).ok_or(KexError::NoCommonEncryptionAlgorithm)?;

                        self.next_kex_at_bytes_sent = t.bytes_sent() + self.interval_bytes;
                        self.next_kex_at_bytes_received = t.bytes_received() + self.interval_bytes;

                        t.consume();
                        return Ok(());
                    }
                    _ => (),
                }
            }
            None => (),
        }
        return Err(KexError::ProtocolError);
    }

    fn poll_flush<T: Socket>(
        &mut self,
        cx: &mut Context,
        t: &mut Transmitter<T>,
    ) -> Poll<Result<(), TransportError>> {
        loop {
            match &mut self.state {
                ClientKexState::Delay(_) => return Poll::Ready(Ok(())),
                ClientKexState::Init(x) => {
                    if !x.sent {
                        ready!(t.poll_send(cx, &x.client_init))?;
                        x.sent = true;
                        match &x.server_init {
                            None => (),
                            Some(server_init) => {
                                self.state = ClientKexState::new_ecdh(
                                    x.client_init.clone(),
                                    server_init.clone(),
                                )?;
                                continue;
                            }
                        }
                    }
                    return Poll::Ready(Ok(()));
                }
                ClientKexState::Ecdh(x) => {
                    if !x.sent {
                        let msg: MsgKexEcdhInit<X25519> =
                            MsgKexEcdhInit::new(X25519::public(&x.dh_secret));
                        ready!(t.poll_send(cx, &msg))?;
                        x.sent = true;
                        break;
                    }
                    return Poll::Ready(Ok(()));
                }
                ClientKexState::NewKeys(x) => {
                    if !x.sent {
                        let msg = MsgNewKeys::new();
                        ready!(t.poll_send(cx, &msg))?;
                        x.sent = true;
                        break;
                    }
                    return Poll::Ready(Ok(()));
                }
            }
        }
        ready!(t.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    // Panics when called before first kex has completed.
    fn session_id(&self) -> &Option<SessionId> {
        &self.session_id
    }
}

fn intersection(preferred: &Vec<&'static str>, supported: &[&'static str]) -> Vec<&'static str> {
    preferred
        .iter()
        .filter_map(|p| {
            supported
                .iter()
                .find_map(|s| if p == s { Some(*s) } else { None })
        })
        .collect::<Vec<&'static str>>()
}

fn common(client: &Vec<&'static str>, server: &Vec<String>) -> Option<&'static str> {
    for c in client {
        for s in server {
            if c == s {
                return Some(*c);
            }
        }
    }
    None
}
