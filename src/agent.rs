mod msg_failure;
mod msg_identities_answer;
mod msg_identities_request;
mod msg_sign_request;
mod msg_sign_response;
mod msg_success;

use self::msg_failure::*;
use self::msg_identities_answer::*;
use self::msg_identities_request::*;
use self::msg_sign_request::*;
use self::msg_sign_response::*;
use crate::algorithm::*;
use crate::transport::{BufferedSender, BufferedReceiver};
use crate::codec::*;
use crate::keys::PublicKey;

use async_std::os::unix::net::UnixStream;
use futures::io::{ReadHalf, WriteHalf, AsyncReadExt};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

pub struct Agent {
    path: PathBuf,
    stream: Option<(BufferedSender<WriteHalf<UnixStream>>, BufferedReceiver<ReadHalf<UnixStream>>)>,
}

impl Agent {
    pub const SSH_AUTH_SOCK: &'static str = "SSH_AUTH_SOCK";

    /// Create a new agent instance by path.
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.into(),
            stream: None,
        }
    }

    /// Create a new agent instance with the value
    /// of SSH_AUTH_SOCK as path.
    pub fn new_env() -> Option<Self> {
        let s = std::env::var_os(Self::SSH_AUTH_SOCK)?;
        Self {
            path: TryFrom::try_from(s).ok()?,
            stream: None,
        }
        .into()
    }

    /// Request a list of identities from the agent.
    pub async fn identities(&self) -> Result<Vec<(PublicKey, String)>, AgentError> {
        let (rh,wh) = UnixStream::connect(&self.path).await?.split();
        let mut s = BufferedSender::new(wh);
        let mut r = BufferedReceiver::new(rh);
        // Send request
        let req = MsgIdentitiesRequest {};
        let len = Encode::size(&req);
        let mut enc = BEncoder::from(s.alloc(4 + len).await?);
        enc.push_u32be(len as u32);
        Encode::encode(&req, &mut enc);
        s.flush().await?;
        // Receive response
        let len = r.read_u32be().await?;
        let buf = r.read_exact(len as usize).await?;
        let mut dec = BDecoder(buf);
        let res: MsgIdentitiesAnswer = Decode::decode(&mut dec).ok_or(AgentError::CodecError)?;
        Ok(res.identities)
    }

    /// Sign a digest with the corresponding private key known to be owned the agent.
    pub async fn sign<'a, S, D>(
        &'a mut self,
        key: &'a S::PublicKey,
        data: &'a D,
        flags: u32,
    ) -> Result<Option<S::Signature>, AgentError>
    where
        S: SignatureAlgorithm,
        S::PublicKey: Encode,
        S::Signature: Decode<'a>,
        D: Encode,
    {
        self.connect().await?;
        match self.stream.as_mut() {
            None => Ok(None),
            Some((s,r)) => {
                // Send request
                let req: MsgSignRequest<S,D> = MsgSignRequest { key, data, flags };
                let len = Encode::size(&req);
                let mut enc = BEncoder::from(s.alloc(4 + len).await?);
                enc.push_u32be(len as u32);
                req.encode(&mut enc);
                s.flush().await?;
                // Receive response
                let len = r.read_u32be().await?;
                let buf = r.read_exact(len as usize).await?;
                let mut dec = BDecoder(&buf[..]);
                let res: E2<MsgSignResponse<S>,MsgFailure> = Decode::decode(&mut dec).ok_or(AgentError::CodecError)?;
                match res {
                    E2::A(x) => Ok(Some(x.signature)),
                    E2::B(_) => Ok(None),
                }
            }
        }
    }

    pub async fn connect(&mut self) -> Result<(),std::io::Error> {
        let (rh,wh) = UnixStream::connect(&self.path).await?.split();
        let mut s = BufferedSender::new(wh);
        let mut r = BufferedReceiver::new(rh);
        self.stream = Some((s,r));
        Ok(())
    }
}

impl Clone for Agent {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            stream: None
        }
    }
}

#[derive(Debug)]
pub enum AgentError {
    CodecError,
    IoError(std::io::Error),
}

impl From<std::io::Error> for AgentError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
