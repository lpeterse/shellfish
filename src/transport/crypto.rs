mod compression;
mod encryption;
mod kex;

pub use self::compression::*;
pub use self::encryption::*;
pub use self::kex::*;

use super::keys::KeyAlgorithm;
use super::MsgKexInit;
use crate::transport::keys::KeyStream;
use crate::transport::TransportError;
use crate::util::secret::Secret;

pub(crate) const MAC_ALGORITHMS: [&'static str; 0] = [];

pub(crate) const COMPRESSION_ALGORITHMS: [&'static str; 1] =
    [<self::compression::NoCompression as CompressionAlgorithm>::NAME];

pub(crate) const ENCRYPTION_ALGORITHMS: [&'static str; 1] =
    [<self::encryption::Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME];

pub fn ciphers<T1, T2>(
    common: fn(&[T2], &[T1]) -> Option<&'static str>,
    alg: KeyAlgorithm,
    server_init: &MsgKexInit<T1>,
    client_init: &MsgKexInit<T2>,
    k: &Secret,
    h: &Secret,
    sid: &Secret,
) -> Result<(CipherConfig, CipherConfig), TransportError> {
    const EENC: TransportError = TransportError::NoCommonEncryptionAlgorithm;
    const ECMP: TransportError = TransportError::NoCommonCompressionAlgorithm;

    let ea_c2s_c = &client_init.encryption_algorithms_client_to_server;
    let ea_c2s_s = &server_init.encryption_algorithms_client_to_server;
    let ea_c2s = common(ea_c2s_c, ea_c2s_s).ok_or(EENC)?;
    let ea_s2c_c = &client_init.encryption_algorithms_server_to_client;
    let ea_s2c_s = &server_init.encryption_algorithms_server_to_client;
    let ea_s2c = common(ea_s2c_c, ea_s2c_s).ok_or(EENC)?;
    let ca_c2s_c = &client_init.compression_algorithms_client_to_server;
    let ca_c2s_s = &server_init.compression_algorithms_client_to_server;
    let ca_c2s = common(ca_c2s_c, ca_c2s_s).ok_or(ECMP)?;
    let ca_s2c_c = &client_init.compression_algorithms_server_to_client;
    let ca_s2c_s = &server_init.compression_algorithms_server_to_client;
    let ca_s2c = common(ca_s2c_c, ca_s2c_s).ok_or(ECMP)?;
    let ma_c2s_c = &client_init.mac_algorithms_client_to_server;
    let ma_c2s_s = &server_init.mac_algorithms_client_to_server;
    let ma_c2s = common(ma_c2s_c, ma_c2s_s);
    let ma_s2c_c = &client_init.mac_algorithms_server_to_client;
    let ma_s2c_s = &server_init.mac_algorithms_server_to_client;
    let ma_s2c = common(ma_s2c_c, ma_s2c_s);
    let ks_c2s = KeyStream::new_c2s(alg, k, h, sid);
    let ks_s2c = KeyStream::new_s2c(alg, k, h, sid);
    let cc_c2s = CipherConfig::new(ea_c2s, ca_c2s, ma_c2s, ks_c2s);
    let cc_s2c = CipherConfig::new(ea_s2c, ca_s2c, ma_s2c, ks_s2c);

    Ok((cc_c2s, cc_s2c))
}

pub fn common(client: &[&'static str], server: &[String]) -> Option<&'static str> {
    for c in client {
        for s in server {
            if c == s {
                return Some(*c);
            }
        }
    }
    None
}

pub fn common_(client: &[String], server: &[&'static str]) -> Option<&'static str> {
    for c in client {
        for s in server {
            if c == s {
                return Some(*s);
            }
        }
    }
    None
}

pub fn intersection(
    preferred: &Vec<&'static str>,
    supported: &[&'static str],
) -> Vec<&'static str> {
    preferred
        .iter()
        .filter_map(|p| {
            supported
                .iter()
                .find_map(|s| if p == s { Some(*s) } else { None })
        })
        .collect::<Vec<&'static str>>()
}
