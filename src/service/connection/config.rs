#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    /// The maximum number of local channels per connection.
    ///
    /// Defaults to 256.
    pub channel_max_count: usize,
    /// The maximum size of input and output buffers (each).
    ///
    /// Defaults to 1MB.
    pub channel_max_buffer_size: usize,
    /// The maximum size of data packets announced to peer.
    ///
    /// Defaults to 32kB.
    pub channel_max_packet_size: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            channel_max_count: 256,
            channel_max_buffer_size: 1024 * 1024,
            channel_max_packet_size: 32768,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_01() {
        let c = ConnectionConfig::default();
        assert_eq!(c.channel_max_count, 256);
        assert_eq!(c.channel_max_buffer_size, 1024 * 1024);
        assert_eq!(c.channel_max_packet_size, 32768);
    }
}
