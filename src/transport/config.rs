use super::identification::Identification;
use crate:: algorithm::*;

use std::time::Duration;

pub struct TransportConfig {
    /// The local identification string.
    /// 
    /// Defaults to `SSH-2.0 ${CARGO_PKG_NAME}_${CARGO_PKG_VERSION}`.
    pub identification: Identification,
    /// The maximum number of bytes (inbound or outbound) after which a rekeying is initiated.
    /// 
    /// Defaults to 1GB (may be capped to an arbitrary value if encryption algorithm demands).
    pub kex_interval_bytes: u64,
    /// The maximum timespan after which a rekeying is initiated.
    /// 
    /// Defaults to 1h.
    pub kex_interval_duration: Duration,
    /// The timespan after which a `MSG_IGNORE` packet is sent when no other data has been sent.
    /// This is useful in order to keep the connection alive in the presence of stateful middle
    /// boxes and firewalls. It will also help to detect broken connections early.
    /// 
    /// Defaults to 5m.
    pub alive_interval: Duration,
    /// The timespan after which the connection is closed due to inactivity when no messages have
    /// been received from peer.
    /// 
    /// Defaults to 1h.
    pub inactivity_timeout: Duration,
    /// List of key exchange algorithms to be used in order of preference.
    /// 
    /// Defaults to `curve25519-sha256` and `curve25519-sha256@libssh.org`.
    pub kex_algorithms: Vec<KexAlgorithm>,
    /// List of host key algorithms to be used in order of preference.
    /// 
    /// Defaults to `ssh-ed25519`.
    pub host_key_algorithms: Vec<HostKeyAlgorithm>,
    /// List of encryption algorithms to be used in order of preference.
    /// 
    /// Defaults to `chacha20-poly1305@openssh.com`.
    pub encryption_algorithms: Vec<EncryptionAlgorithm>,
    /// List of compression algorithms to be used in order of preference.
    /// 
    /// Defaults to `none`.
    pub compression_algorithms: Vec<CompressionAlgorithm>,
    /// List of MAC algorithms to be used in order of preference.
    /// 
    /// Defaults to the empty list.
    pub mac_algorithms: Vec<MacAlgorithm>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            identification: Identification::default(),
            kex_interval_bytes: 1024 * 1024 * 1024,
            kex_interval_duration: Duration::from_secs(3600),
            alive_interval: Duration::from_secs(300),
            inactivity_timeout: Duration::from_secs(3600),
            kex_algorithms: KexAlgorithm::supported(),
            host_key_algorithms: HostKeyAlgorithm::supported(),
            encryption_algorithms: EncryptionAlgorithm::supported(),
            compression_algorithms: CompressionAlgorithm::supported(),
            mac_algorithms: MacAlgorithm::supported(),
        }
    }
}
