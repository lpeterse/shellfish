mod kex_init;
mod kex_ecdh_init;
mod kex_ecdh_reply;
mod kex_hash;

pub use self::kex_init::*;
pub use self::kex_hash::*;
pub use self::kex_ecdh_init::*;
pub use self::kex_ecdh_reply::*;

use super::*;
use crate::algorithm::*;

use rand_os::OsRng;

pub enum Either<A,B> {
    A(A),
    B(B),
}

pub enum KexMessage {
    Init(KexInit),
    EcdhInit(KexEcdhInit),
    EcdhReply(KexEcdhReply),
}

pub enum Kex {
    Invalid,
    ClientWaitingForKex(ClientWaitingForKex),
    ServerWaitingForKex(ServerWaitingForKex),
    ClientWaitingForKexInit(ClientWaitingForKexInit),
    ServerWaitingForKexInit(ServerWaitingForKexInit),
    ClientWaitingForKexEcdhReplyX25519(ClientWaitingForKexEcdhReplyX25519),
}

pub struct ClientWaitingForKex {
    client_identification: Identification,
    server_identification: Identification,
}

pub struct ServerWaitingForKex {
    // TODO
}

pub struct ClientWaitingForKexInit {
    client_identification: Identification,
    server_identification: Identification,
    client_kex_init: KexInit,
}

pub struct ServerWaitingForKexInit {
    client_identification: Identification,
    server_identification: Identification,
    server_kex_init: KexInit,
}

pub struct ClientWaitingForKexEcdhReplyX25519 {
    client_identification: Identification,
    server_identification: Identification,
    client_kex_init: KexInit,
    server_kex_init: KexInit,
    client_dh_secret: x25519_dalek::EphemeralSecret,
    client_dh_public: x25519_dalek::PublicKey,
    encryption_algorithm_client_to_server: EncryptionAlgorithm,
    encryption_algorithm_server_to_client: EncryptionAlgorithm,
}

impl Kex {
    pub fn new_client(client_identification: Identification, server_identification: Identification) -> Self {
        Self::ClientWaitingForKex(ClientWaitingForKex {
            client_identification,
            server_identification,
        })
    }

    pub fn init(&mut self) -> Result<Option<KexInit>, KexError> {
        match std::mem::replace(self, Kex::Invalid) {
            Kex::ClientWaitingForKex(s) => {
                let client_kex_init = KexInit::new(KexCookie::new());
                *self = Kex::ClientWaitingForKexInit(ClientWaitingForKexInit {
                    client_identification: s.client_identification,
                    server_identification: s.server_identification,
                    client_kex_init: client_kex_init.clone(),
                });
                Ok(Some(client_kex_init))
            },
            _ => Err(KexError::ProtocolViolation),
        }
    }

    pub fn push_kex_init(&mut self, server_kex_init: KexInit) -> Result<Option<KexEcdhInit>, KexError> {
        match std::mem::replace(self, Kex::Invalid) {
            Kex::ClientWaitingForKexInit(s) => {
                // Compute the common kex algorithm to use (or error)
                let kex_algorithm = common_algorithm(
                        &s.client_kex_init.kex_algorithms,
                        &server_kex_init.kex_algorithms)
                    .ok_or(KexError::NoCommonKexAlgorithm)?;
                let encryption_algorithm_client_to_server = common_algorithm(
                        &s.client_kex_init.encryption_algorithms_client_to_server,
                        &server_kex_init.encryption_algorithms_client_to_server)
                    .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                let encryption_algorithm_server_to_client = common_algorithm(
                        &s.client_kex_init.encryption_algorithms_server_to_client,
                        &server_kex_init.encryption_algorithms_server_to_client)
                    .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
                // Emit next state and messages based on negotiated kex algorithm
                match kex_algorithm {
                    KexAlgorithm::Curve25519Sha256AtLibsshDotOrg => {
                        let mut csprng: OsRng = OsRng::new().unwrap();
                        let client_dh_secret = x25519_dalek::EphemeralSecret::new(&mut csprng);
                        let client_dh_public = x25519_dalek::PublicKey::from(&client_dh_secret);
                        *self = Kex::ClientWaitingForKexEcdhReplyX25519(ClientWaitingForKexEcdhReplyX25519 {
                            client_identification: s.client_identification,
                            server_identification: s.server_identification,
                            client_kex_init: s.client_kex_init,
                            server_kex_init: server_kex_init,
                            client_dh_secret,
                            client_dh_public,
                            encryption_algorithm_client_to_server,
                            encryption_algorithm_server_to_client,
                        });
                        Ok(Some(KexEcdhInit::new(client_dh_public.as_bytes())))
                    },
                    _ => Err(KexError::KexAlgorithmNotImplemented)
                }
            },
            Kex::ServerWaitingForKexInit(s) => {
                panic!("TODO")
            },
            _ => Err(KexError::ProtocolViolation)
        }
    }

    pub fn push_kex_ecdh_reply(&mut self, ecdh_reply: KexEcdhReply) -> Result<KexOutput, KexError> {
        match std::mem::replace(self, Kex::Invalid) {
            Kex::ClientWaitingForKexEcdhReplyX25519(s) => {
                // Compute the DH shared secret
                let k: [u8;32] = {
                    let key = &ecdh_reply.dh_public;
                    if key.len() != 32 { return Err(KexError::ProtocolViolation) };
                    let mut x: [u8;32] = [0;32];
                    x.copy_from_slice(key);
                    let server_dh_public = x25519_dalek::PublicKey::from(x);
                    s.client_dh_secret.diffie_hellman(&server_dh_public).as_bytes().clone()
                };
                // Compute the exchange hash over the data exchanged so far
                let h: [u8;32] = KexHash {
                    client_identification: &s.client_identification,
                    server_identification: &s.server_identification,
                    client_kex_init: &s.client_kex_init,
                    server_kex_init: &s.server_kex_init,
                    server_host_key: &ecdh_reply.host_key,
                    dh_client_key: s.client_dh_public.as_bytes(),
                    dh_server_key: ecdh_reply.dh_public.as_slice(),
                    dh_secret: &k[..],
                }.as_sha256_digest();
                // Session id is h in first kex
                let sid = h;
                //let keys = KexKeys::new(&k, &h, &sid);

                *self = Kex::ClientWaitingForKex(ClientWaitingForKex {
                    client_identification: s.client_identification,
                    server_identification: s.server_identification,
                });
                Ok(KexOutput { k, h })
            },
            _ => Err(KexError::ProtocolViolation)
        }
    }
}



#[derive(Clone, Debug)]
pub struct KexOutput {
    k: [u8;32],
    h: [u8;32],
}

#[derive(Clone, Debug)]
pub enum KexError {
    ProtocolViolation,
    NoCommonServerHostKeyAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorith,
    KexAlgorithmNotImplemented,
}

fn common_algorithm<T: Clone + PartialEq>(client: &Vec<T>, server: &Vec<T>) -> Option<T> {
    for c in client {
        for s in server {
            if c == s {
                return Some(c.clone())
            }
        }
    }
    None
}
