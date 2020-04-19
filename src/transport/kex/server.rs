use super::kex::*;

pub struct ServerKex {}

impl ServerKex {
    pub fn new() -> Self {
        Self {}
    }
}

impl Kex for ServerKex {
    fn init(&mut self, tx: u64, rx: u64) {
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
    fn poll_init(
        &mut self,
        cx: &mut Context,
        tx: u64,
        rx: u64,
    ) -> Poll<Result<MsgKexInit<&'static str>, TransportError>> {
        todo!()
    }
    fn push_init_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_init_rx(&mut self, tx: u64, rx: u64, msg: MsgKexInit) -> Result<(), TransportError> {
        todo!()
    }

    fn poll_ecdh_init(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<MsgKexEcdhInit<X25519>, TransportError>> {
        todo!()
    }
    fn push_ecdh_init_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_ecdh_init_rx(&mut self, msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        todo!()
    }

    fn poll_ecdh_reply(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<MsgKexEcdhReply<X25519>, TransportError>> {
        todo!()
    }
    fn push_ecdh_reply_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_ecdh_reply_rx(&mut self, msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        todo!()
    }

    fn poll_new_keys_tx(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<EncryptionConfig, TransportError>> {
        todo!()
    }
    fn poll_new_keys_rx(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Result<DecryptionConfig, TransportError>> {
        todo!()
    }
    fn push_new_keys_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_new_keys_rx(&mut self) -> Result<(), TransportError> {
        todo!()
    }

    fn session_id(&self) -> &SessionId {
        unimplemented!()
    }
}
