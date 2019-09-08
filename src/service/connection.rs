mod channel;
mod channel_type;
mod lowest_key_map;
mod msg_channel_open;
mod msg_channel_open_confirmation;
mod msg_channel_open_failure;
mod msg_global_request;

use self::channel::*;
use self::lowest_key_map::*;
use self::msg_channel_open::*;
use self::msg_channel_open_confirmation::*;
use self::msg_channel_open_failure::*;
use self::msg_global_request::*;
use super::user_auth::*;
use crate::codec::*;
use crate::transport::*;

use futures::channel::oneshot;
use futures::channel::mpsc;
use futures::future::TryFutureExt;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use async_std::task;

pub enum Command {
    Disconnect
}

pub struct Connection {
    // Dropping the connection causes the oneshot sender "canary"
    // to be dropped. The handler task is supposed to listen
    // to this with highest priority and terminate itself gracefully.
    _canary: oneshot::Sender<()>,
}

impl Connection {
    pub fn new<T: TransportStream>(t: Transport<T>) -> Self {
        let (s,r) = oneshot::channel();
        task::spawn(async move {
            ConnectionState {
                canary: r,
                transport: t,
                channels: LowestKeyMap::new(256),
            }.run().await
        });
        Connection { _canary: s }
    }
}

pub struct ConnectionState<T> {
    canary: oneshot::Receiver<()>,
    transport: Transport<T>,
    channels: LowestKeyMap<ChannelState>,
}

impl<T: TransportStream> ConnectionState<T> {

    pub async fn run(self) {

    }
    //pub async fn authenticate(mut transport: Transport<T>) -> Result<Self, UserAuthError> {
    //    authenticate(&mut transport).await?;
    //    Ok(ConnectionState {
    //        transport,
    //        channels: LowestKeyMap::new(256),
    //    })
    //}

    pub async fn channel(&mut self) -> Result<Channel, ChannelOpenError> {
        let (s,r) = oneshot::channel();
        self.channels.insert(|_|
            ChannelState::Opening(OpeningChannel { notify:s })
        );
        let req: MsgChannelOpen<'_, Session> = MsgChannelOpen {
            sender_channel: 0,
            initial_window_size: 23,
            maximum_packet_size: 23,
            channel_type: SessionData {},
        };
        self.transport.send(&req).await?;
        self.transport.flush().await?;

        Ok(Channel {})
    }

    async fn send<'a, M: Codec<'a>>(&mut self, msg: &M) -> TransportResult<()> {
        self.transport.send(msg).await
    }

    async fn receive<'a, M: Codec<'a>>(&'a mut self) -> Result<M, TransportError> {
        self.transport.receive().await
    }
}

#[derive(Debug)]
pub struct Channel {}

#[derive(Debug)]
pub struct Process {}

#[derive(Debug)]
pub enum ChannelOpenError {
    TransportError(TransportError),
}

impl From<TransportError> for ChannelOpenError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

#[derive(Debug)]
pub enum SessionError {
    Foobar,
}
