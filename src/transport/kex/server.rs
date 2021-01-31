use super::super::*;

#[derive(Debug)]
pub struct ServerKex;

impl ServerKex {
    pub fn new(_config: &Arc<TransportConfig>, _id: Identification) -> Self {
        unimplemented!()
    }
}

impl Kex for ServerKex {
    fn init(&mut self, _tx: u64, _rx: u64) {
        unimplemented!()
    }
    fn init_if_necessary(&mut self, _cx: &mut Context, _tx: u64, _rx: u64) {
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
    fn peek_init(&mut self, _cx: &mut Context) -> Option<MsgKexInit<&'static str>> {
        todo!()
    }
    fn push_init_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_init_rx(&mut self, _tx: u64, _rx: u64, _msg: MsgKexInit) -> Result<(), TransportError> {
        todo!()
    }

    fn peek_ecdh_init(
        &mut self,
        _cx: &mut Context,
    ) -> Result<Option<MsgKexEcdhInit<X25519>>, TransportError> {
        todo!()
    }
    fn push_ecdh_init_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_ecdh_init_rx(&mut self, _msg: MsgKexEcdhInit<X25519>) -> Result<(), TransportError> {
        todo!()
    }

    fn peek_ecdh_reply(
        &mut self,
        _cx: &mut Context,
    ) -> Result<Option<MsgKexEcdhReply<X25519>>, TransportError> {
        todo!()
    }
    fn push_ecdh_reply_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_ecdh_reply_rx(&mut self, __msg: MsgKexEcdhReply<X25519>) -> Result<(), TransportError> {
        todo!()
    }

    fn poll_new_keys_tx(
        &mut self,
        _cx: &mut Context,
    ) -> Poll<Result<Option<EncryptionConfig>, TransportError>> {
        todo!()
    }
    fn poll_new_keys_rx(
        &mut self,
        _cx: &mut Context,
    ) -> Poll<Result<DecryptionConfig, TransportError>> {
        todo!()
    }
    fn push_new_keys_tx(&mut self) -> Result<(), TransportError> {
        todo!()
    }
    fn push_new_keys_rx(&mut self) -> Result<(), TransportError> {
        todo!()
    }

    fn session_id(&self) -> Result<&SessionId, TransportError> {
        unimplemented!()
    }
}
