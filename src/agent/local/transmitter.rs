use super::error::LocalAgentError;
use super::AuthAgentError;
use super::Frame;
use crate::util::runtime::Socket;
use crate::util::runtime::UnixStream;
use crate::util::codec::*;
use crate::util::check;

pub struct Transmitter<S: Socket = UnixStream> {
    socket: S,
}

impl<S: Socket> Transmitter<S> {
    const MAX_FRAME_LEN: usize = 35000;

    pub async fn send<Msg: SshEncode>(&mut self, msg: &Msg) -> Result<(), AuthAgentError> {
        let vec = SshCodec::encode(&Frame(msg))
            .ok_or(LocalAgentError::EncodingError)
            .map_err(AuthAgentError::new)?;
        self.socket.write_all(&vec).await?;
        Ok(())
    }

    pub async fn receive<Msg: SshDecode>(&mut self) -> Result<Msg, AuthAgentError> {
        let mut len: [u8; 4] = [0; 4];
        self.socket.read_exact(&mut len[..]).await?;
        let len = u32::from_be_bytes(len) as usize;
        check(len <= Self::MAX_FRAME_LEN)
            .ok_or(LocalAgentError::FrameLengthError)
            .map_err(AuthAgentError::new)?;
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, 0);
        self.socket.read_exact(&mut vec[..]).await?;
        SshCodec::decode(&vec)
            .ok_or(LocalAgentError::EncodingError)
            .map_err(AuthAgentError::new)
    }
}

impl<S: Socket> From<S> for Transmitter<S> {
    fn from(socket: S) -> Self {
        Self { socket }
    }
}
