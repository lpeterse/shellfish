use super::msg_channel_close::MsgChannelClose;
use super::msg_channel_data::MsgChannelData;
use super::msg_channel_eof::MsgChannelEof;
use super::msg_channel_extended_data::MsgChannelExtendedData;
use super::msg_channel_failure::*;
use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_channel_success::*;
use super::msg_channel_window_adjust::*;
use super::msg_global_request::*;
use super::*;
use super::{ChannelMap, Connection, ConnectionError, Session};

use crate::requestable;
use crate::transport::*;

use futures::stream::Stream;
use futures::task::{Context, Poll};
use std::pin::*;
use std::sync::{Arc, Mutex};

pub fn poll<T: TransportStream>(
    cx: &mut Context,
    mut transport: Transport<T>,
    requests: &mut requestable::Receiver<Connection>,
    channels: &mut ChannelMap,
) -> Result<Result<Transport<T>, TransportFuture<T>>, ConnectionError> {
    loop {
        match Pin::new(&mut transport).poll_next(cx) {
            Poll::Pending => {
                return Ok(Ok(transport));
            }
            Poll::Ready(None) => {
                return Err(ConnectionError::TransportStreamExhausted);
            }
            Poll::Ready(Some(Err(e))) => {
                return Err(e.into());
            }
            Poll::Ready(Some(Ok(token))) => match transport.redeem_token(token) {
                Some(E10::A(msg)) => {
                    let _: MsgChannelData = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    channel.decrease_local_window_size(msg.data.len())?;
                    match channel.shared {
                        TypedState::Session(ref st) => {
                            let mut shared = st.lock().unwrap();
                            let written = shared.specific.stdout.write(msg.data);
                            assert!(written == msg.data.len());
                            shared.user_task.wake();
                        }
                    }
                    continue;
                }
                Some(E10::B(msg)) => {
                    let _: MsgChannelExtendedData = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    channel.decrease_local_window_size(msg.data.len())?;
                    match channel.shared {
                        TypedState::Session(ref st) => {
                            let mut shared = st.lock().unwrap();
                            let written = shared.specific.stderr.write(msg.data);
                            assert!(written == msg.data.len());
                            shared.user_task.wake();
                        }
                    }
                    continue;
                }
                Some(E10::C(msg)) => {
                    let _: MsgChannelEof = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    match channel.shared {
                        TypedState::Session(ref st) => {
                            let mut shared = st.lock().unwrap();
                            shared.is_remote_eof = true;
                            shared.user_task.wake();
                        }
                    }
                    continue;
                }
                Some(E10::D(msg)) => {
                    let _: MsgChannelClose = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    match channel.shared {
                        TypedState::Session(ref st) => {
                            let mut shared = st.lock().unwrap();
                            if !shared.is_closed {
                                let msg = MsgChannelClose { recipient_channel: channel.remote_channel };
                                match transport.send2(&msg) {
                                    Some(()) => (),
                                    None => return Ok(Err(transport.flush2())),
                                }
                                shared.is_closed = true;
                                shared.user_task.wake();
                            }
                        }
                    }
                    channels.remove(msg.recipient_channel);
                    continue;
                }
                Some(E10::E(msg)) => {
                    log::info!("Ignoring {:?}", msg);
                    let _: MsgGlobalRequest = msg;
                    continue;
                }
                Some(E10::F(msg)) => {
                    let _: MsgChannelOpenConfirmation<Session> = msg;
                    let x: ChannelOpenRequest = requests.take()?;
                    let shared = Arc::new(Mutex::new(Default::default()));
                    let state = ChannelState {
                        is_closing: false,
                        local_channel: msg.recipient_channel,
                        local_window_size: x.initial_window_size,
                        local_max_packet_size: x.max_packet_size,
                        remote_channel: msg.sender_channel,
                        remote_window_size: msg.initial_window_size,
                        remote_max_packet_size: msg.maximum_packet_size,
                        shared: TypedState::Session(shared.clone()),
                    };
                    let session: Session = Session { channel: shared };
                    channels.insert(state)?;
                    requests.respond(ConnectionResponse::OpenSession(session))?;
                    continue;
                }
                Some(E10::G(msg)) => {
                    let _: MsgChannelOpenFailure = msg;
                    let _: ChannelOpenRequest = requests.take()?;
                    requests.respond(ConnectionResponse::OpenFailure(msg.reason))?;
                    continue;
                }
                Some(E10::H(msg)) => {
                    let _: MsgChannelSuccess = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    match channel.shared {
                        TypedState::Session(ref st) => {
                            let mut shared = st.lock().unwrap();
                            shared.specific.request.success()?;
                            shared.user_task.wake();
                        }
                    }
                    continue;
                }
                Some(E10::I(msg)) => {
                    let _: MsgChannelFailure = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    match channel.shared {
                        TypedState::Session(ref st) => {
                            let mut shared = st.lock().unwrap();
                            shared.specific.request.failure()?;
                            shared.user_task.wake();
                        }
                    }
                    continue;
                }
                Some(E10::J(msg)) => {
                    let _: MsgChannelWindowAdjust = msg;
                    let channel = channels.get(msg.recipient_channel)?;
                    channel.remote_window_size += msg.bytes_to_add;
                    continue;
                }
                None => {
                    log::error!("FIXME: unimplemented");
                    continue;
                }
            },
        }
    }
}
