use super::kex::*;

pub struct ClientKexMachine {
    pub state: ClientKexState2,
    pub session_id: SessionId,
    pub interval_bytes: u64,
    pub interval_duration: std::time::Duration,
    pub next_kex_at_bytes_sent: u64,
    pub next_kex_at_bytes_received: u64,
}

pub enum ClientKexState2 {
    Delay(Delay),
    Init(Init),
    Ecdh(Ecdh2<X25519>),
    NewKeys(NewKeys2),
}

pub struct Init {
    pub sent: bool,
    pub client_init: KexInit,
    pub server_init: Option<KexInit>,
}

impl Default for Init {
    fn default() -> Self {
        Self {
            sent: false,
            client_init: KexInit::new(KexCookie::random()),
            server_init: None,
        }
    }
}

pub struct Ecdh2<A: EcdhAlgorithm> {
    pub sent: bool,
    pub client_init: KexInit,
    pub server_init: KexInit,
    pub dh_secret: A::EphemeralSecret,
}

pub struct NewKeys2 {
    pub sent: bool,
    pub client_init: KexInit,
    pub server_init: KexInit,
    pub key_streams: KeyStreams,
}

impl KexMachine for ClientKexMachine {
    fn new(interval_bytes: u64, interval_duration: std::time::Duration) -> Self {
        Self {
            state: ClientKexState2::Init(Init::default()),
            session_id: SessionId::new(),
            interval_bytes,
            interval_duration,
            next_kex_at_bytes_sent: interval_bytes,
            next_kex_at_bytes_received: interval_bytes,
        }
    }

    fn is_init_sent(&self) -> bool {
        log::trace!("is_init_sent");
        match self.state {
            ClientKexState2::Delay(_) => false,
            ClientKexState2::Init(ref x) => x.sent,
            _ => true,
        }
    }

    fn is_init_received(&self) -> bool {
        log::trace!("is_init_received");
        match self.state {
            ClientKexState2::Delay(_) => false,
            ClientKexState2::Init(ref x) => x.server_init.is_some(),
            _ => true,
        }
    }

    fn is_in_progress<T>(
        &mut self,
        cx: &mut Context,
        t: &mut Transmitter<T>,
    ) -> Result<bool, KexError> {
        log::trace!("is_in_progress");
        match &mut self.state {
            ClientKexState2::Delay(timer) => match timer.poll_unpin(cx) {
                Poll::Pending => {
                    if t.bytes_sent >= self.next_kex_at_bytes_sent
                        || t.bytes_received >= self.next_kex_at_bytes_received
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
        log::trace!("init_local");
        match self.state {
            ClientKexState2::Delay(_) => self.state = ClientKexState2::Init(Init::default()),
            _ => (),
        }
    }

    fn init_remote(&mut self, msg: KexInit) -> Result<(), KexError> {
        log::trace!("init_remote");
        match &mut self.state {
            ClientKexState2::Delay(_) => {
                let mut init = Init::default();
                init.server_init = Some(msg);
                self.state = ClientKexState2::Init(init);
            }
            ClientKexState2::Init(init) => {
                if !init.sent {
                    init.server_init = Some(msg);
                } else {
                    let ecdh = Ecdh2 {
                        sent: false,
                        client_init: init.client_init.clone(),
                        server_init: msg,
                        dh_secret: X25519::new(),
                    };
                    self.state = ClientKexState2::Ecdh(ecdh);
                }
            }
            _ => return Err(KexError::ProtocolError),
        }
        Ok(())
    }

    fn consume<T: Socket>(&mut self, t: &mut Transmitter<T>) -> Result<(), KexError> {
        log::trace!("consume");
        match t.decode() {
            Some(msg) => {
                log::debug!("Received MSG_ECDH_REPLY");
                match &mut self.state {
                    ClientKexState2::Ecdh(ecdh) => {
                        let reply: KexEcdhReply<X25519> = msg;
                        // Compute the DH shared secret (create a new placeholder while
                        // the actual secret get consumed in the operation).
                        let dh_secret = std::mem::replace(&mut ecdh.dh_secret, X25519::new());
                        let dh_public = X25519::public(&dh_secret);
                        let k = X25519::diffie_hellman(dh_secret, &reply.dh_public);
                        // Compute the exchange hash over the data exchanged so far.
                        let h: [u8; 32] = KexEcdhHash::<X25519> {
                            client_identification: &t.local_id,
                            server_identification: &t.remote_id,
                            client_kex_init: &ecdh.client_init,
                            server_kex_init: &ecdh.server_init,
                            server_host_key: &reply.host_key,
                            dh_client_key: &dh_public,
                            dh_server_key: &reply.dh_public,
                            dh_secret: X25519::secret_as_ref(&k),
                        }
                        .sha256();
                        // The session id is only computed during first kex and constant afterwards.
                        self.session_id.set_if_uninitialized(h);
                        self.state = ClientKexState2::NewKeys(NewKeys2 {
                            sent: false,
                            client_init: ecdh.client_init.clone(),
                            server_init: ecdh.server_init.clone(),
                            key_streams: KeyStreams::new_sha256(
                                X25519::secret_as_ref(&k),
                                &h,
                                self.session_id,
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
                let _: NewKeys = msg;
                log::debug!("Received MSG_NEW_KEYS");
                let state = ClientKexState2::Delay(Delay::new(self.interval_duration));
                let state = std::mem::replace(&mut self.state, state);
                match state {
                    ClientKexState2::NewKeys(mut x) => {
                        let encryption_algorithm_client_to_server = common_algorithm(
                            &x.client_init.encryption_algorithms_client_to_server,
                            &x.server_init.encryption_algorithms_client_to_server,
                        )
                        .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                        let encryption_algorithm_server_to_client = common_algorithm(
                            &x.client_init.encryption_algorithms_server_to_client,
                            &x.server_init.encryption_algorithms_server_to_client,
                        )
                        .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                        let compression_algorithm_client_to_server = common_algorithm(
                            &x.client_init.compression_algorithms_client_to_server,
                            &x.server_init.compression_algorithms_client_to_server,
                        )
                        .ok_or(KexError::NoCommonCompressionAlgorithm)?;
                        let compression_algorithm_server_to_client = common_algorithm(
                            &x.client_init.compression_algorithms_server_to_client,
                            &x.server_init.compression_algorithms_server_to_client,
                        )
                        .ok_or(KexError::NoCommonCompressionAlgorithm)?;
                        let mac_algorithm_client_to_server = common_algorithm(
                            &x.client_init.mac_algorithms_client_to_server,
                            &x.server_init.mac_algorithms_client_to_server,
                        );
                        let mac_algorithm_server_to_client = common_algorithm(
                            &x.client_init.mac_algorithms_server_to_client,
                            &x.server_init.mac_algorithms_server_to_client,
                        );

                        t.encryption_ctx.new_keys(
                            &encryption_algorithm_client_to_server,
                            &compression_algorithm_client_to_server,
                            &mac_algorithm_client_to_server,
                            &mut x.key_streams.c(),
                        );
                        t.decryption_ctx.new_keys(
                            &encryption_algorithm_server_to_client,
                            &compression_algorithm_server_to_client,
                            &mac_algorithm_server_to_client,
                            &mut x.key_streams.d(),
                        );

                        self.next_kex_at_bytes_sent = t.bytes_sent + self.interval_bytes;
                        self.next_kex_at_bytes_received = t.bytes_received + self.interval_bytes;

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
        log::trace!("poll_flush");
        match &mut self.state {
            ClientKexState2::Delay(_) => return Poll::Ready(Ok(())),
            ClientKexState2::Init(x) => {
                if x.sent {
                    return Poll::Ready(Ok(()));
                } else {
                    let msg = x.client_init.clone();
                    ready!(t.poll_send(cx, &msg))?;
                    x.sent = true;
                }
            }
            ClientKexState2::Ecdh(x) => {
                if x.sent {
                    return Poll::Ready(Ok(()));
                } else {
                    let msg: KexEcdhInit<X25519> = KexEcdhInit::new(X25519::public(&x.dh_secret));
                    ready!(t.poll_send(cx, &msg))?;
                    x.sent = true;
                }
            }
            ClientKexState2::NewKeys(x) => {
                if x.sent {
                    return Poll::Ready(Ok(()));
                } else {
                    let msg = NewKeys::new();
                    ready!(t.poll_send(cx, &msg))?;
                    x.sent = true;
                }
            }
        }
        ready!(t.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    // Panics when called before first kex has completed.
    fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}
