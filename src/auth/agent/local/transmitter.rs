use super::*;
use crate::util::socket::Socket;
use crate::util::*;

use async_std::io::prelude::WriteExt;
use async_std::os::unix::net::UnixStream;

pub struct Transmitter<S: Socket = UnixStream> {
    socket: S,
}

impl<S: Socket> Transmitter<S> {
    const MAX_FRAME_LEN: usize = 35000;

    pub async fn send<Msg: SshEncode>(&mut self, msg: &Msg) -> Result<(), AgentError> {
        let vec = SshCodec::encode(&Frame(msg)).ok_or("encoding failed")?;
        self.socket.write_all(&vec).await?;
        Ok(())
    }

    pub async fn receive<Msg: SshDecode>(&mut self) -> Result<Msg, AgentError> {
        let mut len: [u8; 4] = [0; 4];
        self.socket.read_exact(&mut len[..]).await?;
        let len = u32::from_be_bytes(len) as usize;
        check(len <= Self::MAX_FRAME_LEN).ok_or("MAX_FRAME_LEN exceeded")?;
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, 0);
        self.socket.read_exact(&mut vec[..]).await?;
        SshCodec::decode(&vec).ok_or("decoding failed".into())
    }
}

impl<S: Socket> From<S> for Transmitter<S> {
    fn from(socket: S) -> Self {
        Self { socket }
    }
}
