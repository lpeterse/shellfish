mod cookie;
mod ecdh_init;
mod ecdh_reply;
mod ecdh_hash;
mod init;
mod new_keys;

pub use self::cookie::*;
pub use self::ecdh_hash::*;
pub use self::ecdh_init::*;
pub use self::ecdh_reply::*;
pub use self::init::*;
pub use self::new_keys::*;

use super::*;
use crate::algorithm::*;

use rand_os::OsRng;

pub struct Ecdh {
    client_kex_init: KexInit,
    server_kex_init: KexInit,
    client_dh_init: KexEcdhInit,
    client_dh_secret: x25519_dalek::EphemeralSecret,
    client_dh_public: x25519_dalek::PublicKey,
    encryption_algorithm_client_to_server: EncryptionAlgorithm,
    encryption_algorithm_server_to_client: EncryptionAlgorithm,
    compression_algorithm_client_to_server: CompressionAlgorithm,
    compression_algorithm_server_to_client: CompressionAlgorithm,
    mac_algorithm_client_to_server: Option<MacAlgorithm>,
    mac_algorithm_server_to_client: Option<MacAlgorithm>,
}

impl Ecdh {
    pub fn new(client_kex_init: KexInit, server_kex_init: KexInit) -> Result<Ecdh, KexError> {
        // Compute the common algorithms, abort on mismatch
        let kex_algorithm = common_algorithm(
                &client_kex_init.kex_algorithms,
                &server_kex_init.kex_algorithms)
            .ok_or(KexError::NoCommonKexAlgorithm)?;

        let encryption_algorithm_client_to_server = common_algorithm(
                &client_kex_init.encryption_algorithms_client_to_server,
                &server_kex_init.encryption_algorithms_client_to_server)
            .ok_or(KexError::NoCommonEncryptionAlgorithm)?;
        let encryption_algorithm_server_to_client = common_algorithm(
                &client_kex_init.encryption_algorithms_server_to_client,
                &server_kex_init.encryption_algorithms_server_to_client)
            .ok_or(KexError::NoCommonEncryptionAlgorithm)?;

        let compression_algorithm_client_to_server = common_algorithm(
                &client_kex_init.compression_algorithms_client_to_server,
                &server_kex_init.compression_algorithms_client_to_server)
            .ok_or(KexError::NoCommonCompressionAlgorithm)?;
        let compression_algorithm_server_to_client = common_algorithm(
                &client_kex_init.compression_algorithms_server_to_client,
                &server_kex_init.compression_algorithms_server_to_client)
            .ok_or(KexError::NoCommonCompressionAlgorithm)?;

        let mac_algorithm_client_to_server = common_algorithm(
                &client_kex_init.mac_algorithms_client_to_server,
                &server_kex_init.mac_algorithms_client_to_server);
        let mac_algorithm_server_to_client = common_algorithm(
                &client_kex_init.mac_algorithms_server_to_client,
                &server_kex_init.mac_algorithms_server_to_client);

        // Emit next state and messages based on negotiated kex algorithm
        match kex_algorithm {
            KexAlgorithm::Curve25519Sha256AtLibsshDotOrg => {
                let mut csprng: OsRng = OsRng::new().unwrap();
                let client_dh_secret = x25519_dalek::EphemeralSecret::new(&mut csprng);
                let client_dh_public = x25519_dalek::PublicKey::from(&client_dh_secret);
                let client_dh_init = KexEcdhInit::new(client_dh_public.as_bytes());
                Ok(Ecdh {
                    client_kex_init: client_kex_init,
                    server_kex_init: server_kex_init,
                    client_dh_init,
                    client_dh_secret,
                    client_dh_public,
                    mac_algorithm_client_to_server,
                    mac_algorithm_server_to_client,
                    encryption_algorithm_client_to_server,
                    encryption_algorithm_server_to_client,
                    compression_algorithm_client_to_server,
                    compression_algorithm_server_to_client,
                })
            },
            _ => panic!("kex algorithm not supported")
        }
    }

    pub fn init(&self) -> &KexEcdhInit {
        &self.client_dh_init
    }

    pub fn reply(self, ecdh_reply: KexEcdhReply, client_id: &Identification, server_id: &Identification, session_id: &SessionId) -> Result<KexOutput, KexError> {
        // Compute the DH shared secret
        let k: [u8;32] = {
            let key = &ecdh_reply.dh_public;
            if key.len() != 32 { return Err(KexError::DecoderError) };
            let mut x: [u8;32] = [0;32];
            x.copy_from_slice(key);
            let server_dh_public = x25519_dalek::PublicKey::from(x);
            self.client_dh_secret.diffie_hellman(&server_dh_public).as_bytes().clone()
        };
        // Compute the exchange hash over the data exchanged so far
        let h: [u8;32] = KexEcdhHash {
            client_identification: &client_id,
            server_identification: &server_id,
            client_kex_init: &self.client_kex_init,
            server_kex_init: &self.server_kex_init,
            server_host_key: &ecdh_reply.host_key,
            dh_client_key: self.client_dh_public.as_bytes(),
            dh_server_key: ecdh_reply.dh_public.as_slice(),
            dh_secret: &k[..],
        }.sha256();

        let key_streams = if session_id.is_uninitialized() {
            KeyStreams::new_sha256(&k, &h, &h)
        } else {
            KeyStreams::new_sha256(&k, &h, session_id)
        };

        Ok(KexOutput {
            session_id: if session_id.is_uninitialized() { Some(SessionId::from(h)) } else { None },
            key_streams,
            encryption_algorithm_client_to_server: self.encryption_algorithm_client_to_server,
            encryption_algorithm_server_to_client: self.encryption_algorithm_server_to_client,
            compression_algorithm_client_to_server: self.compression_algorithm_client_to_server,
            compression_algorithm_server_to_client: self.compression_algorithm_server_to_client,
            mac_algorithm_client_to_server: self.mac_algorithm_client_to_server,
            mac_algorithm_server_to_client: self.mac_algorithm_server_to_client,
        })
    }
}

#[derive(Debug)]
pub struct KexOutput {
    pub session_id: Option<SessionId>,
    pub key_streams: KeyStreams,
    pub encryption_algorithm_client_to_server: EncryptionAlgorithm,
    pub encryption_algorithm_server_to_client: EncryptionAlgorithm,
    pub compression_algorithm_client_to_server: CompressionAlgorithm,
    pub compression_algorithm_server_to_client: CompressionAlgorithm,
    pub mac_algorithm_client_to_server: Option<MacAlgorithm>,
    pub mac_algorithm_server_to_client: Option<MacAlgorithm>,
}

#[derive(Clone, Debug)]
pub enum KexError {
    DecoderError,
    NoCommonServerHostKeyAlgorithm,
    NoCommonCompressionAlgorithm,
    NoCommonEncryptionAlgorithm,
    NoCommonKexAlgorithm,
    NoCommonMacAlgorith,
    InvalidSignature,
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
