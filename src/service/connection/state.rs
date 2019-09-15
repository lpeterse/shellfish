use super::*;
use crate::transport::*;
use super::msg_global_request::*;

use futures::channel::oneshot;
use futures::channel::mpsc;
use futures::stream::StreamExt;
use futures::FutureExt;
use futures::select;

pub struct ConnectionState<T> {
    pub canary: oneshot::Receiver<()>,
    pub commands: mpsc::Receiver<Command>,
    pub transport: Transport<T>,
    pub channels: LowestKeyMap<ChannelState>,
}

impl<T: TransportStream> ConnectionState<T> {

    pub async fn run(mut self) -> Result<(),ConnectionError> {
        enum Event<T> {
            Command(Command),
            Message(T),
        }

        loop {
            let event = {
                let t1 = self.commands.next();
                let t2 = self.transport.receive().fuse();
                futures::pin_mut!( t1, t2 );
                futures::select! {
                    x = t1 => {
                        Event::Command(x.ok_or(ConnectionError::CommandStreamTerminated)?)
                    },
                    x = t2 =>  {
                        Event::Message(x?)
                    },
                    complete => break
                }
            };
            match event {
                Event::Command(cmd) => self.dispatch_command(cmd).await?,
                Event::Message(_) => println!("MESSAGE"),
            }
        }
        Ok(())
    }

    pub async fn dispatch_command(&mut self, cmd: Command) -> Result<(), ConnectionError> {
        match cmd {
            Command::ChannelOpenSession(x) => println!("ASHDA"),
            Command::Foobar => println!("FOOBAR"),
        }
        Ok(())
    }

    pub async fn dispatch_message<'a>(&'a mut self, msg: Message<'a>) -> Result<(), ConnectionError> {
        Ok(())
    }

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
}

#[derive(Debug)]
pub enum Message<'a> {
    GlobalRequest(MsgGlobalRequest<'a>)
}

#[derive(Debug)]
pub enum ConnectionError {
    ConnectionLost,
    CommandStreamTerminated,
    TransportError(TransportError),
    ChannelOpenFailure(ChannelOpenFailure),
}

impl From<TransportError> for ConnectionError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<ChannelOpenFailure> for ConnectionError {
    fn from(e: ChannelOpenFailure) -> Self {
        Self::ChannelOpenFailure(e)
    }
}
