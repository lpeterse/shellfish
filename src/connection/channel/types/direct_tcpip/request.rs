pub enum DirectTcpIpRequest {}

impl ChannelRequest for DirectTcpIpRequest {
    fn name(&self) -> &'static str {
        unreachable!()
    }
}

impl SshEncode for DirectTcpIpRequest {
    fn encode<E: SshEncoder>(&self, _e: &mut E) {
        unreachable!()
    }
}
