use super::AuthAgentError;
use super::Frame;
use crate::util::check;
use crate::util::codec::*;
use crate::util::socket::{read_exact, write_all, Socket};
use tokio::net::UnixStream;

pub struct Transmitter<S: Socket = UnixStream> {
    socket: S,
}

impl<S: Socket> Transmitter<S> {
    const MAX_FRAME_LEN: usize = 35000;

    pub async fn send<Msg: SshEncode>(&mut self, msg: &Msg) -> Result<(), AuthAgentError> {
        let vec = SshCodec::encode(&Frame(msg)).map_err(AuthAgentError::new)?;
        write_all(&mut self.socket, &vec).await?;
        Ok(())
    }

    pub async fn receive<Msg: SshDecode>(&mut self) -> Result<Msg, AuthAgentError> {
        let mut len: [u8; 4] = [0; 4];
        read_exact(&mut self.socket, &mut len[..]).await?;
        let len = u32::from_be_bytes(len) as usize;
        check(len <= Self::MAX_FRAME_LEN)
            .ok_or(SshCodecError::DecodingFailed)
            .map_err(AuthAgentError::new)?;
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, 0);
        read_exact(&mut self.socket, &mut vec[..]).await?;
        SshCodec::decode(&vec).map_err(AuthAgentError::new)
    }
}

impl<S: Socket> From<S> for Transmitter<S> {
    fn from(socket: S) -> Self {
        Self { socket }
    }
}
