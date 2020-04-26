use super::*;
use crate::algorithm::*;

use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TransportConfig {
    /// The local identification string.
    ///
    /// Defaults to `SSH-2.0-${CARGO_PKG_NAME}_${CARGO_PKG_VERSION}`.
    pub identification: Identification<&'static str>,
    /// The maximum number of bytes (inbound or outbound) after which a rekeying is initiated.
    ///
    /// Defaults to 1GB (may be capped to an arbitrary value if encryption algorithm demands).
    pub kex_interval_bytes: u64,
    /// The maximum timespan after which a rekeying is initiated.
    ///
    /// Defaults to 1h.
    pub kex_interval_duration: Duration,
    /// List of key exchange algorithms to be used in order of preference.
    ///
    /// Defaults to `curve25519-sha256` and `curve25519-sha256@libssh.org`.
    pub kex_algorithms: Vec<&'static str>,
    /// List of host key authenticaton algorithms to be used in order of preference.
    ///
    /// Defaults to `ssh-ed25519`.
    pub host_key_algorithms: Vec<&'static str>,
    /// List of encryption algorithms to be used in order of preference.
    ///
    /// Defaults to `chacha20-poly1305@openssh.com`.
    pub encryption_algorithms: Vec<&'static str>,
    /// List of compression algorithms to be used in order of preference.
    ///
    /// Defaults to `none`.
    pub compression_algorithms: Vec<&'static str>,
    /// List of MAC algorithms to be used in order of preference.
    ///
    /// Defaults to the empty list.
    pub mac_algorithms: Vec<&'static str>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            identification: Identification::default(),
            kex_interval_bytes: 1024 * 1024 * 1024,
            kex_interval_duration: Duration::from_secs(3600),
            kex_algorithms: KEX_ALGORITHMS.to_vec(),
            host_key_algorithms: HOST_KEY_ALGORITHMS.to_vec(),
            encryption_algorithms: ENCRYPTION_ALGORITHMS.to_vec(),
            compression_algorithms: COMPRESSION_ALGORITHMS.to_vec(),
            mac_algorithms: MAC_ALGORITHMS.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_01() {
        let c = TransportConfig::default();
        assert_eq!(c.identification, Identification::default());
        assert_eq!(c.kex_interval_bytes, 1024 * 1024 * 1024);
        assert_eq!(c.kex_interval_duration, Duration::from_secs(3600));
        assert_eq!(c.kex_algorithms, KEX_ALGORITHMS.to_vec());
        assert_eq!(c.host_key_algorithms, HOST_KEY_ALGORITHMS.to_vec());
        assert_eq!(c.encryption_algorithms, ENCRYPTION_ALGORITHMS.to_vec());
        assert_eq!(c.compression_algorithms, COMPRESSION_ALGORITHMS.to_vec());
        assert_eq!(c.mac_algorithms, MAC_ALGORITHMS.to_vec());
    }
}
