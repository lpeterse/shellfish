use super::*;
use super::channel::*;
use super::msg_channel_close::*;
use super::msg_channel_open::*;
use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_global_request::*;
use crate::transport::*;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::ready;
use futures::future::Future;
use futures::stream::Stream;
use futures::task::{Context, Poll};
use std::pin::*;

pub struct ConnectionFuture<T> {
    pub error: Option<oneshot::Sender<ConnectionError>>,
    pub command: Option<Command>,
    pub commands: mpsc::Receiver<Command>,
    pub transport: TransportFuture<T>,
    pub channels: LowestKeyMap<ChannelState>,
}

impl<T> ConnectionFuture<T> {
    pub fn new(
        error: oneshot::Sender<ConnectionError>,
        commands: mpsc::Receiver<Command>,
        transport: Transport<T>,
    ) -> Self {
        Self {
            error: Some(error),
            command: None,
            commands,
            transport: TransportFuture::Ready(transport),
            channels: LowestKeyMap::new(256),
        }
    }

    pub fn send_error(&mut self, e: ConnectionError) {
        let error = std::mem::replace(&mut self.error, None).unwrap();
        error.send(e).unwrap_or(());
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
            //===================================================================================//
            // CHECK FUTURE FOR READYNESS (ERROR OCCURED OR CONNECTION DROPPED)                  //
            //===================================================================================//

            match &mut self_.error {
                None => {
                    log::debug!("Ready: Error transmitted");
                    return Poll::Ready(());
                }
                Some(error) => {
                    match Pin::new(error).poll_cancel(cx) {
                        Poll::Pending => (), // fall through
                        Poll::Ready(()) => {
                            log::debug!("Ready: Connection dropped");
                            return Poll::Ready(());
                        }
                    }
                }
            }

            //===================================================================================//
            // POLL TRANSPORT FUTURE AND EVENTUALLY ACQUIRE TRANSPORT FOR SUBSEQUENT USE         //
            //===================================================================================//

            log::debug!("Poll transport future");
            let mut transport = match ready!(Pin::new(&mut self_.transport).poll(cx)) {
                Ok(t) => t,
                Err(e) => {
                    self_.send_error(e.into());
                    continue;
                }
            };

            //===================================================================================//
            // POLL & HANDLE INCOMING TRANSPORT MESSAGES                                         //
            //===================================================================================//

            log::debug!("Poll transport stream");
            match Pin::new(&mut transport).poll_next(cx) {
                Poll::Pending => (), // fall through
                Poll::Ready(None) => {
                    self_.send_error(ConnectionError::TransportStreamExhausted);
                    continue;
                }
                Poll::Ready(Some(Err(e))) => {
                    self_.send_error(e.into());
                    continue;
                }
                Poll::Ready(Some(Ok(token))) => match transport.redeem_token(token) {
                    Some(E3::A(msg)) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgGlobalRequest = msg;
                        self_.transport = transport.future();
                        continue;
                    }
                    Some(E3::B(msg)) => {
                        log::debug!("Received MsgChannelOpenConfirmation");
                        let _: MsgChannelOpenConfirmation<Session> = msg;
                        match self_.channels.get_mut(msg.recipient_channel as usize) {
                            None => {
                                self_.send_error(ConnectionError::InvalidChannelId);
                                continue;
                            }
                            Some(state) => {
                                let (s, r) = mpsc::channel(1);
                                let open = Open {
                                    local_channel: msg.recipient_channel,
                                    local_initial_window_size: 128, // TODO
                                    local_max_packet_size: 128,     // TODO
                                    remote_channel: msg.sender_channel,
                                    remote_initial_window_size: msg.initial_window_size,
                                    remote_max_packet_size: msg.maximum_packet_size,
                                    receive_buffer: std::collections::VecDeque::new(), // TODO
                                    notify: s,
                                };
                                match std::mem::replace(state, ChannelState::Open(open)) {
                                    ChannelState::Opening(reply) => {
                                        // Notify the channel handler.
                                        // It is safe to ignore the error in case of a dropped
                                        // handler here. Dead channels will be detected and
                                        // handled below.
                                        reply
                                            .send(Ok(Channel {
                                                id: msg.recipient_channel,
                                                request: (),
                                                confirmation: (),
                                                notification: r,
                                            }))
                                            .unwrap_or(());
                                        self_.transport = transport.future();
                                        continue;
                                    }
                                    _ => {
                                        self_.send_error(ConnectionError::InvalidChannelState);
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    Some(E3::C(msg)) => {
                        log::debug!("Received MsgChannelOpenFailure");
                        let _: MsgChannelOpenFailure = msg;
                        match self_.channels.remove(msg.recipient_channel as usize) {
                            Some(ChannelState::Opening(reply)) => {
                                // Notify the channel handler. It is safe to ignore
                                // the error as all resources have been deallocated.
                                reply
                                    .send(Err(OpenFailure {
                                        reason: msg.reason,
                                        description: msg.description,
                                    }))
                                    .unwrap_or(());
                                self_.transport = transport.future();
                                continue;
                            }
                            Some(_) => {
                                self_.send_error(ConnectionError::InvalidChannelState);
                                continue;
                            }
                            None => {
                                self_.send_error(ConnectionError::InvalidChannelId);
                                continue;
                            }
                        }
                    }
                    None => {
                        log::error!("FIXME: unimplemented");
                        self_.transport = transport.future();
                        continue;
                    }
                },
            }

            //===================================================================================//
            // POLL & HANDLE COMMANDS                                                            //
            //===================================================================================//

            let cmd = match std::mem::replace(&mut self_.command, None) {
                Some(cmd) => {
                    log::debug!("Retry pending command");
                    Some(cmd)
                }
                None => {
                    log::debug!("Poll command stream");
                    match Pin::new(&mut self_.commands).poll_next(cx) {
                        Poll::Pending => {
                            log::debug!("No command");
                            None
                        }
                        Poll::Ready(None) => {
                            self_.send_error(ConnectionError::CommandStreamExhausted);
                            continue;
                        }
                        Poll::Ready(Some(cmd)) => Some(cmd),
                    }
                }
            };

            match cmd {
                None => (), // fall through
                Some(Command::Debug(msg)) => {
                    log::debug!("Command::Debug");
                    let msg = MsgDebug::new(msg.clone());
                    match transport.send2(&msg) {
                        Some(()) => {
                            self_.transport = transport.future();
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
                            self_.transport = transport.future();
                            continue;
                        }
                        None => {
                            log::debug!("Need to flush first");
                            self_.transport = transport.flush2();
                            continue;
                        }
                    }
                }
                Some(Command::ChannelOpenSession(reply)) => {
                    log::debug!("Command::ChannelOpenSession");
                    match self_.channels.free_key() {
                        None => {
                            // In case of local channel shortage, reject the request.
                            // It is safe to do nothing if the reply channel was dropped
                            // in the meantime as no resources have been allocated.
                            reply
                                .send(Err(OpenFailure {
                                    reason: ChannelOpenFailureReason::RESOURCE_SHORTAGE,
                                    description: "".into(),
                                }))
                                .unwrap_or(());
                            self_.transport = transport.future();
                            continue;
                        }
                        Some(key) => {
                            let msg: MsgChannelOpen<Session> = MsgChannelOpen {
                                sender_channel: key as u32,
                                initial_window_size: 23,
                                maximum_packet_size: 23,
                                channel_type: (),
                            };
                            match transport.send2(&msg) {
                                Some(()) => {
                                    self_.channels.insert2(key, ChannelState::Opening(reply));
                                    self_.transport = transport.future();
                                    continue;
                                }
                                None => {
                                    log::debug!("Need to flush first");
                                    self_.command = Some(Command::ChannelOpenSession(reply));
                                    self_.transport = transport.flush2();
                                    continue;
                                }
                            }
                        }
                    }
                }
            }

            //===================================================================================//
            // DETECT AND HANDLE DEAD CHANNELS                                                   //
            //===================================================================================//

            // Determine the first dead channel id (if any).
            // As a side effect, looping through and polling all channels registers them with
            // the scheduler.
            let dead_channel_id = {
                let mut iter = self_.channels.into_iter();
                loop {
                    match iter.next() {
                        Some(ChannelState::Open(x)) => {
                            match Pin::new(&mut x.notify).poll_ready(cx) {
                                Poll::Ready(Err(_)) => break Some(x.local_channel),
                                _ => continue,
                            }
                        }
                        // Opening channels will become open and collected later.
                        // Closing channels will be deallocated as soon as confirmed by peer.
                        Some(_) => continue,
                        None => break None,
                    }
                }
            };

            match dead_channel_id {
                None => (), // fall through
                Some(id) => match self_.channels.get_mut(id as usize) {
                    Some(st) => match st {
                        ChannelState::Open(channel) => {
                            let msg = MsgChannelClose {
                                recipient_channel: channel.remote_channel,
                            };
                            match transport.send2(&msg) {
                                Some(()) => {
                                    std::mem::replace(st, ChannelState::Closing);
                                    self_.transport = transport.future();
                                    continue;
                                }
                                None => {
                                    self_.transport = transport.flush2();
                                    continue;
                                }
                            }
                        }
                        _ => {
                            self_.send_error(ConnectionError::InvalidChannelState);
                            continue;
                        }
                    },
                    _ => {
                        self_.send_error(ConnectionError::InvalidChannelId);
                        continue;
                    }
                },
            }

            //===================================================================================//
            // FLUSH TRANSPORT IF NECESSARY OR RETURN PENDING                                    //
            //===================================================================================//

            if !transport.flushed() {
                self_.transport = transport.flush2();
                continue;
            } else {
                self_.transport = transport.future();
                return Poll::Pending;
            }
        }
    }
}


