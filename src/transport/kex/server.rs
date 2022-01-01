use super::ciphers;
use super::common_;
use super::config::TransportConfig;
use super::error::TransportError;
use super::ident::Identification;
use super::keys::KeyAlgorithm;
use super::msg::*;
use super::CipherConfig;
use super::Curve25519Sha256;
use super::Kex;
use super::KexAlgorithm;
use super::KexCookie;
use super::KexHash;
use super::KexMessage;
use crate::agent::AuthAgent;
use crate::agent::AuthAgentFuture;
use crate::identity::Signature;
use crate::util::check;
use crate::util::secret::Secret;
use crate::{identity::Identity, ready};
use core::future::Future;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

const EIST: TransportError = TransportError::InvalidState;
const EENC: TransportError = TransportError::InvalidEncoding;
const ESIG: TransportError = TransportError::AgentRefusedToSign;
const EAHK: TransportError = TransportError::NoCommonServerHostKeyAlgorithm;
const EAKX: TransportError = TransportError::NoCommonKexAlgorithm;

macro_rules! isset {
    ( $s:expr, $bit:ident ) => {
        $s.state & State::$bit != 0
    };
}

macro_rules! nisset {
    ( $s:expr, $bit:ident ) => {
        $s.state & State::$bit == 0
    };
}

macro_rules! set {
    ( $s:expr, $bit:ident ) => {
        $s.state |= State::$bit
    };
}

pub struct ServerKex {
    config: Arc<TransportConfig>,
    /// Authentication agent (identity)
    agent: Arc<dyn AuthAgent>,
    /// Remote identification string
    client_id: Identification<String>,
    /// Session id (only after after initial kex, constant afterwards)
    session_id: Option<Secret>,
    /// Mutable state (when kex in progress)
    state: Option<Box<State>>,
    /// Output buffer
    output: VecDeque<KexMessage>,
}

impl ServerKex {
    pub fn new(
        config: &Arc<TransportConfig>,
        agent: &Arc<dyn AuthAgent>,
        client_id: Identification<String>,
    ) -> Box<dyn Kex> {
        Box::new(Self {
            config: config.clone(),
            agent: agent.clone(),
            client_id,
            session_id: None,
            state: None,
            output: VecDeque::new(),
        })
    }
}

impl Kex for ServerKex {
    fn init(&mut self) {
        if self.state.is_none() {
            self.state = Some(Box::new(State::default()));
        }
    }

    fn push_init(&mut self, msg: MsgKexInit) -> Result<(), TransportError> {
        self.init();
        let s = self.state.as_mut().ok_or(EIST)?;
        check(nisset!(s, INIT_RCVD)).ok_or(EIST)?;
        s.client_init = Some(Arc::new(msg));
        set!(s, INIT_RCVD);
        Ok(())
    }

    fn push_ecdh_init(&mut self, msg: MsgEcdhInit) -> Result<(), TransportError> {
        let s = self.state.as_mut().ok_or(EIST)?;
        check(isset!(s, INIT_RCVD)).ok_or(EIST)?;
        check(nisset!(s, ECDH_RCVD)).ok_or(EIST)?;
        s.client_ecdh_init = Some(Arc::new(msg));
        set!(s, ECDH_RCVD);
        Ok(())
    }

    fn push_new_keys(&mut self) -> Result<Box<CipherConfig>, TransportError> {
        let s = self.state.as_mut().ok_or(EIST)?;
        check(isset!(s, ECDH_SENT)).ok_or(EIST)?;
        check(isset!(s, ECDH_RCVD)).ok_or(EIST)?;
        check(nisset!(s, KEYS_RCVD)).ok_or(EIST)?;
        set!(s, KEYS_RCVD);
        Ok(s.cipher_c2s.take().ok_or(EIST)?)
    }

    fn session_id(&self) -> Option<&Secret> {
        self.session_id.as_ref()
    }

    fn poll(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<&mut VecDeque<KexMessage>, TransportError>> {
        if let Some(s) = self.state.as_mut() {
            if nisset!(s, HOST_KEYS) {
                s.server_host_keys_fut = Some(self.agent.identities());
                set!(s, HOST_KEYS);
            }

            if nisset!(s, INIT_SENT) {
                let fut = s.server_host_keys_fut.as_mut().ok_or(EIST)?;
                let ids = ready!(Pin::new(fut).poll(cx))?;
                let cnf = &self.config;
                let cki = KexCookie::random();
                let fka = |a: &str| ids.iter().any(|b| a == b.0.algorithm());
                let msg = MsgKexInit::new_from_config(cki, cnf).restrict_hka(fka)?;
                let msg = Arc::new(msg);
                self.output.push_back(KexMessage::Init(msg.clone()));
                s.server_host_keys_fut = None;
                s.server_host_keys = Some(ids);
                s.server_init = Some(msg);
                set!(s, INIT_SENT);
            }

            if nisset!(s, ECDH_EVAL) && isset!(s, INIT_SENT) && isset!(s, ECDH_RCVD) {
                let ki_srv = s.server_init.as_ref().ok_or(EIST)?;
                let ki_cli = s.client_init.as_ref().ok_or(EIST)?;
                let ei_cli = s.client_ecdh_init.as_ref().ok_or(EIST)?;
                let ka_cli = &ki_cli.kex_algorithms;
                let ka_srv = &ki_srv.kex_algorithms;
                let ka = common_(ka_cli, ka_srv);
                let hka_cli = &ki_cli.server_host_key_algorithms;
                let hka_srv = &ki_srv.server_host_key_algorithms;
                let hka = common_(hka_cli, hka_srv).ok_or(EAHK)?;
                let hks = s.server_host_keys.as_ref().ok_or(EIST)?;
                let hk = &hks.iter().find(|id| id.0.algorithm() == hka).ok_or(EAHK)?.0;

                match ka {
                    Some(Curve25519Sha256::NAME) => {
                        use rand_core::OsRng;
                        let dh_sec_srv = x25519_dalek::EphemeralSecret::new(&mut OsRng);
                        let dh_pub_srv = x25519_dalek::PublicKey::from(&dh_sec_srv);
                        let dh_pub_cli = TryInto::<[u8; 32]>::try_into(&ei_cli.dh_public[..]);
                        let dh_pub_cli = dh_pub_cli.ok().ok_or(EENC)?;
                        let dh_pub_cli = x25519_dalek::PublicKey::from(dh_pub_cli);
                        // Compute the shared secret
                        let k = Secret::new(dh_sec_srv.diffie_hellman(&dh_pub_cli).as_bytes());
                        // Compute the exchange hash over the data exchanged so far
                        let h: Secret = KexHash::<_, _> {
                            client_id: &self.client_id,
                            server_id: &self.config.identification,
                            client_kex_init: &ki_cli,
                            server_kex_init: &ki_srv,
                            server_host_key: hk,
                            dh_client_key: dh_pub_cli.as_bytes(),
                            dh_server_key: dh_pub_srv.as_bytes(),
                            dh_secret: &k,
                        }
                        .sha256();
                        let sid = self.session_id.get_or_insert_with(|| h.clone());
                        let alg = KeyAlgorithm::Sha256;
                        let (c2s, s2c) = ciphers(common_, alg, ki_srv, ki_cli, &k, &h, &sid)?;
                        s.server_host_key = Some(hk.clone());
                        s.server_ecdh_pub = Some(dh_pub_srv.to_bytes().into());
                        s.server_signature = Some(self.agent.signature(hk, h.as_ref(), 0));
                        s.cipher_c2s = Some(Box::new(c2s));
                        s.cipher_s2c = Some(Box::new(s2c));
                        s.client_init = None;
                        s.server_init = None;
                        s.server_host_keys = None;
                    }
                    _ => Err(EAKX)?,
                }
                set!(s, ECDH_EVAL)
            }

            if nisset!(s, ECDH_SENT) && isset!(s, ECDH_EVAL) {
                let fut = s.server_signature.as_mut().ok_or(EIST)?;
                let sig = ready!(Pin::new(fut).poll(cx))?.ok_or(ESIG)?;
                let shk = s.server_host_key.take().ok_or(EIST)?;
                let sdp = s.server_ecdh_pub.take().ok_or(EIST)?;
                let s2c = s.cipher_s2c.take().ok_or(EIST)?;
                let msg1 = MsgEcdhReply::new(shk, sdp, sig);
                s.server_signature = None;
                self.output.push_back(KexMessage::EcdhReply(Arc::new(msg1)));
                self.output.push_back(KexMessage::NewKeys(s2c));
                set!(s, ECDH_SENT);
                set!(s, KEYS_SENT);
            }

            if isset!(s, KEYS_SENT) && isset!(s, KEYS_RCVD) {
                self.state = None;
            }
        }

        Poll::Ready(Ok(&mut self.output))
    }
}

impl std::fmt::Debug for ServerKex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServerKex {{ ... }}")
    }
}

#[derive(Default)]
struct State {
    state: u8,
    client_init: Option<Arc<MsgKexInit<String>>>,
    client_ecdh_init: Option<Arc<MsgEcdhInit>>,
    server_init: Option<Arc<MsgKexInit<&'static str>>>,
    server_host_keys_fut: Option<AuthAgentFuture<Vec<(Identity, String)>>>,
    server_host_keys: Option<Vec<(Identity, String)>>,
    server_host_key: Option<Identity>,
    server_ecdh_pub: Option<Vec<u8>>,
    server_signature: Option<AuthAgentFuture<Option<Signature>>>,
    cipher_c2s: Option<Box<CipherConfig>>,
    cipher_s2c: Option<Box<CipherConfig>>,
}

impl State {
    const HOST_KEYS: u8 = 1;
    const INIT_SENT: u8 = 2;
    const INIT_RCVD: u8 = 4;
    const ECDH_RCVD: u8 = 8;
    const ECDH_EVAL: u8 = 16;
    const ECDH_SENT: u8 = 32;
    const KEYS_SENT: u8 = 64;
    const KEYS_RCVD: u8 = 128;
}
