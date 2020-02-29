use super::kex::*;

pub struct ServerKex {}

impl ServerKex {
    pub fn new() -> Self {
        Self {}
    }
}

impl Kex for ServerKex {
    fn init(&mut self) {
        unimplemented!()
    }
    fn is_active(&self) -> bool {
        unimplemented!()
    }
    fn is_sending_critical(&self) -> bool {
        unimplemented!()
    }
    fn is_receiving_critical(&self) -> bool {
        unimplemented!()
    }
    fn push_init(&mut self, msg: MsgKexInit) -> Result<(), TransportError> {
        unimplemented!()
    }
    fn push_ecdh_init(&mut self, msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        unimplemented!()
    }
    fn push_ecdh_reply(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        unimplemented!()
    }
    fn push_new_keys(&mut self) -> Result<CipherConfig, TransportError> {
        unimplemented!()
    }
    fn poll<F>(
        &mut self,
        cx: &mut Context,
        bytes_sent: u64,
        bytes_received: u64,
        f: F,
    ) -> Poll<Result<(), TransportError>>
    where
        F: FnMut(&mut Context, KexOutput) -> Poll<Result<(), TransportError>>,
    {
        unimplemented!()
    }
    fn session_id(&self) -> &SessionId {
        unimplemented!()
    }
}
