use super::*;
use crate::util::codec::*;

#[derive(Debug)]
pub struct ChannelOpenRequest {
    tx: OpenInboundTx,
    name: String,
    data: Vec<u8>,
    chan: ChannelHandle,
}

impl ChannelOpenRequest {
    pub(crate) fn new(name: String, data: Vec<u8>, chan: ChannelHandle) -> (Self, OpenInboundRx) {
        let (tx, rx) = oneshot::channel();
        let s = Self {
            tx,
            name,
            data,
            chan,
        };
        (s, rx)
    }

    pub fn is<D: Channel>(&self) -> bool {
        D::NAME == self.name
    }

    pub fn data<D: Channel>(&self) -> Result<D::Open, ConnectionError> {
        if self.is::<D>() {
            let e = ConnectionError::ChannelTypeMismatch;
            return Ok(SshCodec::decode(&self.data).ok_or(e)?);
        }
        Err(ConnectionError::ChannelTypeMismatch)
    }

    pub fn accept<D: Channel>(self) -> Result<D, ConnectionError> {
        if self.is::<D>() {
            self.tx.send(Ok(()));
            return Ok(D::new(self.chan));
        }
        Err(ConnectionError::ChannelTypeMismatch)
    }

    pub fn reject(self, reason: ChannelOpenFailure) {
        self.tx.send(Err(reason))
    }
}
