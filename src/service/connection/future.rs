use super::channel::*;
use super::msg_channel_open::*;
use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_global_request::*;
use super::*;
use crate::transport::*;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::Either;
use futures::future::Future;
use futures::ready;
use futures::select;
use futures::stream::{Stream, StreamExt};
use futures::task::{Context, Poll};
use futures::FutureExt;
use std::pin::*;

pub struct ConnectionFuture<T> {
    pub canary: oneshot::Receiver<()>,
    pub commands: mpsc::Receiver<Command>,
    pub command_pending: Option<Command>,
    pub transport: TransportFuture<T>,
    pub channels: LowestKeyMap<ChannelState>,
}

impl<T> ConnectionFuture<T> {
    pub fn new(
        canary: oneshot::Receiver<()>,
        commands: mpsc::Receiver<Command>,
        transport: Transport<T>,
    ) -> Self {
        Self {
            canary,
            commands,
            command_pending: None,
            transport: TransportFuture::Ready(transport),
            channels: LowestKeyMap::new(256),
        }
    }
}

impl<T> Future for ConnectionFuture<T>
where
    T: Unpin + TransportStream,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        log::debug!("ConnectionFuture.poll()");
        let mut self_ = Pin::into_inner(self);
        loop {
            log::debug!("Check canary");
            match self_.canary.try_recv() {
                Ok(None) => (), // fall through
                _ => {
                    log::debug!("Canary dropped or fired");
                    return Poll::Ready(());
                }
            }

            log::debug!("Poll transport future");
            let mut transport = match ready!(Pin::new(&mut self_.transport).poll(cx)) {
                Ok(t) => t,
                Err(e) => {
                    log::warn!("Transport error: {:?}", e);
                    return Poll::Ready(());
                }
            };

            log::debug!("Poll transport stream");
            match Pin::new(&mut transport).poll_next(cx) {
                Poll::Pending => (),
                Poll::Ready(None) => {
                    log::debug!("Transport stream exhausted");
                    return Poll::Ready(());
                }
                Poll::Ready(Some(Err(e))) => {
                    log::debug!("Transport error: {:?}", e);
                    return Poll::Ready(());
                }
                Poll::Ready(Some(Ok(token))) => match transport.redeem_token(token) {
                    Some(msg) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgGlobalRequest = msg;
                        self_.transport = transport.future();
                        continue;
                    }
                    None => {
                        log::error!("FIXME: unimplemented");
                        self_.transport = transport.future();
                        continue;
                    }
                },
            }

            if self_.command_pending.is_none() {
                log::debug!("Poll command stream");
                match Pin::new(&mut self_.commands).poll_next(cx) {
                    Poll::Pending => (), // fall through
                    Poll::Ready(None) => {
                        log::debug!("Command stream exhausted");
                        return Poll::Ready(());
                    }
                    Poll::Ready(cmd) => {
                        self_.command_pending = cmd
                    }
                }
            }

            match &self_.command_pending {
                None => (), // fall through
                Some(Command::Debug(msg)) => {
                    log::debug!("Command::Debug");
                    let msg = MsgDebug::new(msg.clone());
                    match transport.send2(&msg) {
                        Some(()) => {
                            self_.command_pending = None;
                            self_.transport = transport.flush2();
                            continue;
                        }
                        None => {
                            log::debug!("Need to flush first");
                            self_.transport = transport.flush2();
                            continue;
                        }
                    }
                }
                Some(Command::Disconnect) => {
                    log::debug!("Command::Disconnect");
                    let msg = MsgDisconnect::by_application("FOOOBAR".into());
                    match transport.send2(&msg) {
                        Some(()) => {
                            self_.command_pending = None;
                            self_.transport = transport.flush2();
                            continue;
                        }
                        None => {
                            log::debug!("Need to flush first");
                            self_.transport = transport.flush2();
                            continue;
                        }
                    }
                }
                Some(Command::ChannelOpenSession(x)) => {
                    log::debug!("Command::ChannelOpenSession");
                    self_.transport = transport.future();
                    continue;
                    /*
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
                        */
                },
            }

            log::debug!("Store idle transport");
            self_.transport = transport.future();
            return Poll::Pending;
        }
    }
}

/*

impl ConnectionState {
    pub async fn run<T: TransportStream>(mut self, transport: Transport<T>, commands: mpsc::Receiver<Command>) -> Result<(), ConnectionError> {
        enum Event<T> {
            Command(Command),
            Message(T),
        }

        loop {
            let t1 = transport.next();

        }
        /*
        let r = transport.for_each(commands, |transport, input| Box::pin( async {
            if true {
                Ok(Either::Left(transport))
            } else {
                Ok(Either::Right(()))
            }
            /*
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
            }*/
        })).await;

        log::error!("RRRRR {:?}", r);
        */
        /*
        loop {
            log::error!("LOOP");
            let event = {
                let t1 = self.commandself_.next();
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
}*/

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
