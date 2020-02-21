use super::*;

use async_std::os::unix::net::UnixStream;
use futures::io::{AsyncReadExt, AsyncWriteExt};

pub struct Transmitter {
    stream: UnixStream,
}

impl Transmitter {
    const MAX_PACKET_LEN: usize = 35000;

    pub async fn new(path: &PathBuf) -> Result<Self, AgentError> {
        Ok(Self {
            stream: UnixStream::connect(&path).await?,
        })
    }

    pub async fn send<Msg: Encode>(&mut self, msg: &Msg) -> Result<(), AgentError> {
        let vec = BEncoder::encode(&Frame::new(&msg));
        self.stream.write_all(&vec).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn receive<Msg: Decode>(&mut self) -> Result<Msg, AgentError> {
        let mut len: [u8; 4] = [0; 4];
        self.stream.read_exact(&mut len[..]).await?;
        let len = u32::from_be_bytes(len) as usize;
        assert!(len <= Self::MAX_PACKET_LEN);
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, 0);
        self.stream.read_exact(&mut vec[..]).await?;
        BDecoder::decode(&vec).ok_or(AgentError::DecoderError)
    }
}
