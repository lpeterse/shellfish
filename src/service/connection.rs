mod channel;
mod command;
mod error;
mod future;
mod lowest_key_map;
mod msg_channel_close;
mod msg_channel_open;
mod msg_channel_open_confirmation;
mod msg_channel_open_failure;
mod msg_global_request;

use self::channel::*;
use self::error::*;
use self::lowest_key_map::*;
//use self::msg_channel_open::*;
//use self::msg_channel_open_confirmation::*;
//use self::msg_channel_open_failure::*;
//use self::msg_global_request::*;
use self::command::*;
use self::future::ConnectionFuture;
use super::user_auth::*;
use super::*;
use crate::agent::*;
use crate::codec::*;
use crate::transport::*;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::TryFutureExt;
use futures::sink::SinkExt;

pub struct Connection {
    error: oneshot::Receiver<ConnectionError>,
    command: mpsc::Sender<Command>,
}

impl Connection {
    pub fn new<T: TransportStream>(t: Transport<T>) -> Connection {
        let (s1, r1) = oneshot::channel();
        let (s2, r2) = mpsc::channel(1);
        async_std::task::spawn(ConnectionFuture::new(s1, r2, t));
        Connection {
            command: s2,
            error: r1,
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), ChannelOpenError> {
        self.command
            .send(Command::Disconnect)
            .map_err(|_| ChannelOpenError::ConnectionLost)
            .await
    }

    pub async fn debug(&mut self, msg: String) -> Result<(), ChannelOpenError> {
        self.command
            .send(Command::Debug(msg))
            .map_err(|_| ChannelOpenError::ConnectionLost)
            .await
    }

    pub async fn open_session(&mut self) -> Result<Session, ChannelOpenError> {
        let (s, r) = oneshot::channel();
        let request = Command::ChannelOpenSession(s);
        self.command
            .send(request)
            .map_err(|_| ChannelOpenError::ConnectionLost)
            .await?;
        let response = r
            .map_err(|e| {
                log::error!("AHAJSDHAKSH {:?}", e);
                ChannelOpenError::ConnectionLost
            })
            .await?;
        Ok(Session {})
    }
}

impl Service for Connection {
    const NAME: &'static str = "ssh-connection";
}

#[derive(Debug)]
pub enum ChannelOpenError {
    ConnectionLost,
    TransportError(TransportError),
    ChannelOpenFailure(ChannelOpenFailure),
}

impl From<TransportError> for ChannelOpenError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<ChannelOpenFailure> for ChannelOpenError {
    fn from(e: ChannelOpenFailure) -> Self {
        Self::ChannelOpenFailure(e)
    }
}
