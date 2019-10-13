pub trait ConnectionConfig {
    fn channel_max_count(&self) -> u32;
}
