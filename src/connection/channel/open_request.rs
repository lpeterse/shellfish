use super::*;
use crate::util::codec::*;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct ChannelOpenRequest {
    pub name: String,
    pub data: Vec<u8>,
    pub chan: ChannelHandle,
    pub resp: oneshot::Sender<Result<(), ChannelOpenFailure>>
}

impl ChannelOpenRequest {
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
            self.resp.send(Ok(())).unwrap_or(());
            return Ok(D::new(self.chan));
        }
        Err(ConnectionError::ChannelTypeMismatch)
    }

    pub fn reject(self, reason: ChannelOpenFailure) {
        self.resp.send(Err(reason)).unwrap_or(())
    }
}
