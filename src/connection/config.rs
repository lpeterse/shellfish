#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    /// The maximum number of queued requests/replies per connection.
    ///
    /// This is not an actually allocated queue size, but a limit on the summed
    /// length of all queues at which the connection shall safely terminate with
    /// an error instead of performing unbounded allocation in order to avoid
    /// DoS attacks.
    ///
    /// Defaults to 256.
    pub queued_max_count: u32,
    /// The maximum number of local channels per connection.
    ///
    /// Defaults to 256.
    pub channel_max_count: u32,
    /// The maximum size of all buffers combined per channel.
    ///
    /// Defaults to 1MB.
    pub channel_max_buffer_size: u32,
    /// The maximum size of data packets announced to peer.
    ///
    /// Defaults to 32kB.
    pub channel_max_packet_size: u32,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            queued_max_count: 256,
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
        assert_eq!(c.queued_max_count, 256);
        assert_eq!(c.channel_max_count, 256);
        assert_eq!(c.channel_max_buffer_size, 1024 * 1024);
        assert_eq!(c.channel_max_packet_size, 32768);
    }
}
