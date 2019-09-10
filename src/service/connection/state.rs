use super::*;
use crate::codec::*;
use crate::transport::*;

use futures::channel::oneshot;
use futures::channel::mpsc;
use futures::future::TryFutureExt;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use async_std::task;

pub struct ConnectionState<T> {
    pub canary: oneshot::Receiver<()>,
    pub commands: mpsc::Receiver<Command>,
    pub transport: Transport<T>,
    pub channels: LowestKeyMap<ChannelState>,
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

    /*
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
    }*/

    async fn send<'a, M: Codec<'a>>(&mut self, msg: &M) -> TransportResult<()> {
        self.transport.send(msg).await
    }

    async fn receive<'a, M: Codec<'a>>(&'a mut self) -> Result<M, TransportError> {
        self.transport.receive().await
    }
}