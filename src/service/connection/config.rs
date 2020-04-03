pub trait ConnectionConfig {
    fn channel_max_count(&self) -> usize;
    fn channel_max_buffer_size(&self) -> usize;
    fn channel_max_packet_size(&self) -> usize;
}
