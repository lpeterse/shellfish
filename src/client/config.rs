use crate::algorithm::*;
use crate::transport::*;
use crate::service::connection::ConnectionConfig;

use std::time::Duration;

pub struct ClientConfig {
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
    /// The maximum number of local channels.
    /// 
    /// Defaults to 256.
    pub channel_max_count: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            identification: Identification::default(),
            kex_interval_bytes: 1024 * 1024 * 1024,
            kex_interval_duration: Duration::from_secs(3600),
            alive_interval: Duration::from_secs(300),
            inactivity_timeout: Duration::from_secs(3600),
            kex_algorithms: SUPPORTED_KEX_ALGORITHMS.to_vec(),
            host_key_algorithms: SUPPORTED_HOST_KEY_ALGORITHMS.to_vec(),
            encryption_algorithms: SUPPORTED_ENCRYPTION_ALGORITHMS.to_vec(),
            compression_algorithms: SUPPORTED_COMPRESSION_ALGORITHMS.to_vec(),
            mac_algorithms: SUPPORTED_MAC_ALGORITHMS.to_vec(),
            channel_max_count: 256,
        }
    }
}

impl TransportConfig for ClientConfig {
    fn identification(&self) -> &Identification<&'static str> {
        &self.identification
    }
    fn kex_interval_bytes(&self) -> u64 {
        self.kex_interval_bytes
    }
    fn kex_interval_duration(&self) -> Duration {
        self.kex_interval_duration
    }
    fn alive_interval(&self) -> Duration {
        self.alive_interval
    }
    fn inactivity_timeout(&self) -> Duration {
        self.inactivity_timeout
    }
    fn kex_algorithms(&self) -> &Vec<&'static str> {
        &self.kex_algorithms
    }
    fn host_key_algorithms(&self) -> &Vec<&'static str> {
        &self.host_key_algorithms
    }
    fn encryption_algorithms(&self) -> &Vec<&'static str> {
        &self.encryption_algorithms
    }
    fn compression_algorithms(&self) -> &Vec<&'static str> {
        &self.compression_algorithms
    }
    fn mac_algorithms(&self) -> &Vec<&'static str> {
        &self.mac_algorithms
    }
}

impl ConnectionConfig for ClientConfig {
    fn channel_max_count(&self) -> u32 {
        self.channel_max_count
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_01() {
        let c = ClientConfig::default();
        assert_eq!(c.identification(), &Identification::default());
        assert_eq!(c.kex_interval_bytes(), 1024 * 1024 * 1024);
        assert_eq!(c.kex_interval_duration(), Duration::from_secs(3600));
        assert_eq!(c.alive_interval(), Duration::from_secs(300));
        assert_eq!(c.inactivity_timeout(), Duration::from_secs(3600));
        assert_eq!(c.kex_algorithms(), &SUPPORTED_KEX_ALGORITHMS.to_vec());
        assert_eq!(c.host_key_algorithms(), &SUPPORTED_HOST_KEY_ALGORITHMS.to_vec());
        assert_eq!(c.encryption_algorithms(), &SUPPORTED_ENCRYPTION_ALGORITHMS.to_vec());
        assert_eq!(c.compression_algorithms(), &SUPPORTED_COMPRESSION_ALGORITHMS.to_vec());
        assert_eq!(c.mac_algorithms(), &SUPPORTED_MAC_ALGORITHMS.to_vec());
        assert_eq!(c.channel_max_count(), 256);
    }
}
