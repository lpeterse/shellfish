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
use crate::client::*;
use crate::codec::*;
use crate::keys::PublicKey;
use crate::role::*;
use crate::transport::buffered_receiver::BufferedReceiver;
use crate::transport::buffered_sender::BufferedSender;

use async_std::os::unix::net::UnixStream;
use futures::io::{AsyncReadExt, ReadHalf, WriteHalf};
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub struct Agent<R: Role> {
    phantom: std::marker::PhantomData<R>,
    path: PathBuf,
}

impl<R: Role> Agent<R> {
    const SSH_AUTH_SOCK: &'static str = "SSH_AUTH_SOCK";
}

impl Agent<Client> {
    /// Create a new agent instance by path.
    pub fn new(path: &Path) -> Self {
        Self {
            phantom: std::marker::PhantomData,
            path: path.into(),
        }
    }

    /// Create a new agent instance with the value
    /// of `SSH_AUTH_SOCK` as path.
    pub fn new_env() -> Option<Self> {
        let s = std::env::var_os(Self::SSH_AUTH_SOCK)?;
        Self {
            phantom: std::marker::PhantomData,
            path: TryFrom::try_from(s).ok()?,
        }
        .into()
    }

    /// Request a list of identities from the agent.
    pub async fn identities(&self) -> Result<Vec<(PublicKey, String)>, AgentError> {
        let (mut s, mut r) = self.connect().await?;
        // Send request
        let req = MsgIdentitiesRequest {};
        let len = Encode::size(&req);
        let mut enc = BEncoder::from(s.reserve(4 + len).await?);
        enc.push_u32be(len as u32);
        Encode::encode(&req, &mut enc);
        s.flush().await?;
        // Receive response
        let len = r.read_u32be().await?;
        let buf = r.read_exact(len as usize).await?;
        let mut dec = BDecoder(buf);
        let res: MsgIdentitiesAnswer =
            DecodeRef::decode(&mut dec).ok_or(Error::new(ErrorKind::InvalidData, ""))?;
        Ok(res.identities)
    }

    /// Sign a digest with the corresponding private key known to be owned the agent.
    pub async fn sign<S, D>(
        &self,
        key: &S::PublicKey,
        data: &D,
        flags: S::SignatureFlags,
    ) -> Result<Option<S::Signature>, AgentError>
    where
        S: SignatureAlgorithm,
        S::PublicKey: Encode,
        S::Signature: Decode,
        D: Encode,
    {
        let (mut s, mut r) = self.connect().await?;
        // Send request
        let req: MsgSignRequest<S, D> = MsgSignRequest { key, data, flags };
        let len = Encode::size(&req);
        let mut enc = BEncoder::from(s.reserve(4 + len).await?);
        enc.push_u32be(len as u32);
        req.encode(&mut enc);
        s.flush().await?;
        // Receive response
        let len = r.read_u32be().await?;
        let buf = r.read_exact(len as usize).await?;
        let mut dec = BDecoder(&buf[..]);
        let res: E2<MsgSignResponse<S>, MsgFailure> =
            DecodeRef::decode(&mut dec).ok_or(Error::new(ErrorKind::InvalidData, ""))?;
        match res {
            E2::A(x) => Ok(Some(x.signature)),
            E2::B(_) => Ok(None),
        }
    }

    async fn connect(
        &self,
    ) -> Result<
        (
            BufferedSender<WriteHalf<UnixStream>>,
            BufferedReceiver<ReadHalf<UnixStream>>,
        ),
        Error,
    > {
        let (rh, wh) = UnixStream::connect(&self.path).await?.split();
        let s = BufferedSender::new(wh);
        let r = BufferedReceiver::new(rh);
        Ok((s, r))
    }
}

impl<R: Role> Clone for Agent<R> {
    fn clone(&self) -> Self {
        Self {
            phantom: std::marker::PhantomData,
            path: self.path.clone(),
        }
    }
}

#[derive(Debug)]
pub enum AgentError {
    IoError(Error),
}

impl From<Error> for AgentError {
    fn from(e: Error) -> Self {
        Self::IoError(e)
    }
}
