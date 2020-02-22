use super::identification::Identification;

use std::time::Duration;

pub trait TransportConfig {
    fn identification(&self) -> &Identification<&'static str>;
    fn identification_timeout(&self) -> Duration;
    fn kex_interval_bytes(&self) -> u64;
    fn kex_interval_duration(&self) -> Duration;
    fn alive_interval(&self) -> Duration;
    fn inactivity_timeout(&self) -> Duration;
    fn kex_algorithms(&self) -> &Vec<&'static str>;
    fn host_key_algorithms(&self) -> &Vec<&'static str>;
    fn encryption_algorithms(&self) -> &Vec<&'static str>;
    fn compression_algorithms(&self) -> &Vec<&'static str>;
    fn mac_algorithms(&self) -> &Vec<&'static str>;
}
