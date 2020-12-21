

pub enum DirectTcpIpRequest {}

impl ChannelRequest for DirectTcpIpRequest {
    fn name(&self) -> &'static str {
        unreachable!()
    }
}

impl Encode for DirectTcpIpRequest {
    fn size(&self) -> usize {
        unreachable!()
    }

    fn encode<E: SshEncoder>(&self, _e: &mut E) {
        unreachable!()
    }
}