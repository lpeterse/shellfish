use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_channel_open::*;
use super::msg_global_request::*;
use super::channel::*;
use super::*;
use crate::transport::*;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::select;
use futures::stream::StreamExt;
use futures::FutureExt;
use futures::future::Either;

pub struct ConnectionState {
    pub canary: oneshot::Receiver<()>,
    //pub commands: mpsc::Receiver<Command>,
    //pub transport: Transport<T>,
    pub channels: LowestKeyMap<ChannelState>,
}

impl ConnectionState {
    pub async fn run<T: TransportStream>(mut self, transport: Transport<T>, commands: mpsc::Receiver<Command>) -> Result<(), ConnectionError> {
        enum Event<T> {
            Command(Command),
            Message(T),
        }
        let r = transport.for_each(commands, |transport, input| Box::pin( async {
            match input {
                Either::Left(token) => {
                    log::error!("TOKEN {:?}", token);
                    match transport.redeem_token(token).await? {
                        None => {
                            log::error!("REDEEM FAILED");
                            ()
                        },
                        Some(msg) => {
                            let _: MsgGlobalRequest = msg;
                            log::error!("Ignoring {:?}", msg);
                            ()
                        }
                    }
                    /*
                    */
                    Ok(Some(()))
                },
                Either::Right(event) => {
                    log::error!("EVENT");
                    Ok(Some(()))
                }
            }
        })).await;

        log::error!("RRRRR {:?}", r);
        /*
        loop {
            log::error!("LOOP");
            let event = {
                let t1 = self.commands.next();
                let t2 = self.transport.try_receive().fuse();
                futures::pin_mut!(t1, t2);
                futures::select! {
                    x = t1 => {
                        Event::Command(x.ok_or(ConnectionError::CommandStreamTerminated)?)
                    },
                    x = t2 =>  {
                        match x? {
                            None => continue,
                            Some(x) => Event::Message(x),
                        }
                    },
                    complete => break
                }
            };
            match event {
                Event::Command(cmd) => match cmd {
                    Command::ChannelOpenSession(x) => {
                        match self.channels.insert(ChannelState::Opening(x)) {
                            Ok(id) => {
                                let req: MsgChannelOpen<Session> = MsgChannelOpen {
                                    sender_channel: id as u32,
                                    initial_window_size: 23,
                                    maximum_packet_size: 23,
                                    channel_type: (),
                                };
                                self.transport.send(&req).await?;
                                self.transport.flush().await?;
                                log::error!("BBBBBBB {}", id);
                            }
                            Err(ChannelState::Opening(x)) => {
                                // In case of local channel shortage, reject the request.
                                // It is safe to do nothing if the reply channel was dropped
                                // in the meantime as no resources have been allocated.
                                x.send(Err(OpenFailure {
                                    reason: ChannelOpenFailureReason::RESOURCE_SHORTAGE,
                                    description: "".into()
                                })).unwrap_or(())
                            }
                            _ => panic!("ABC")
                        }
                    }
                    Command::Foobar => println!("FOOBAR"),
                },
                Event::Message(msg) => match msg {
                    E3::A(msg) => {
                        let _: MsgGlobalRequest = msg;
                        log::info!("Ignoring {:?}", msg);
                    }
                    E3::B(msg) => {
                        let _: MsgChannelOpenConfirmation<Session> = msg;
                        
                        println!("OPPPPPPPEEEEENNNNN");
                    }
                    E3::C(msg) => {
                        log::error!("FAILURE");
                        let _: MsgChannelOpenFailure = msg;
                        match self.channels.get(msg.recipient_channel as usize) {
                            None => return Err(ConnectionError::InvalidChannelId),
                            Some(c) => panic!("")
                        }
                        println!("MESSAGE");
                    }
                },
            }
        }*/
        Ok(())
    }
}

#[derive(Debug)]
pub enum Message<'a> {
    GlobalRequest(MsgGlobalRequest<'a>),
}

#[derive(Debug)]
pub enum ConnectionError {
    ConnectionLost,
    CommandStreamTerminated,
    InvalidChannelId,
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
