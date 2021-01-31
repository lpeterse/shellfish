use super::*;

#[derive(Debug)]
pub struct HostKeys;

impl Global for HostKeys {
    const NAME: &'static str = "hostkeys-00@openssh.com";
    type RequestData = Vec<u8>;
}
