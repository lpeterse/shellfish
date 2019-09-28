use super::msg_channel_close::MsgChannelClose;
use super::msg_channel_data::MsgChannelData;
use super::msg_channel_eof::MsgChannelEof;
use super::msg_channel_extended_data::MsgChannelExtendedData;
use super::msg_channel_failure::*;
use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_channel_success::*;
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
    log::debug!("Poll transport stream");
    loop {
        match Pin::new(&mut transport).poll_next(cx) {
            Poll::Pending => {
                log::info!("TRANSPORT STREAM PENDING");
                return Ok(Ok(transport));
            }
            Poll::Ready(None) => {
                return Err(ConnectionError::TransportStreamExhausted);
            }
            Poll::Ready(Some(Err(e))) => {
                return Err(e.into());
            }
            Poll::Ready(Some(Ok(token))) => {
                log::info!("GOT TOKEN");
                match transport.redeem_token(token) {
                    Some(E9::A(msg)) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgChannelData = msg;
                        continue;
                    }
                    Some(E9::B(msg)) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgChannelExtendedData = msg;
                        continue;
                    }
                    Some(E9::C(msg)) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgChannelEof = msg;
                        continue;
                    }
                    Some(E9::D(msg)) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgChannelClose = msg;
                        continue;
                    }
                    Some(E9::E(msg)) => {
                        log::info!("Ignoring {:?}", msg);
                        let _: MsgGlobalRequest = msg;
                        continue;
                    }
                    Some(E9::F(msg)) => {
                        log::debug!("Received MsgChannelOpenConfirmation");
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
                        channels.insert(msg.recipient_channel as usize, state);
                        requests.respond(ConnectionResponse::OpenSession(session))?;
                        continue;
                    }
                    Some(E9::G(msg)) => {
                        log::debug!("Received MsgChannelOpenFailure");
                        let _: MsgChannelOpenFailure = msg;
                        let _: ChannelOpenRequest = requests.take()?;
                        requests.respond(ConnectionResponse::OpenFailure(msg.reason))?;
                        continue;
                    }
                    Some(E9::H(msg)) => {
                        log::debug!("Received MsgChannelSuccess");
                        let _: MsgChannelSuccess = msg;
                        //let _: ChannelOpenRequest = requests.take()?;
                        //requests.respond(ConnectionResponse::OpenFailure(msg.reason))?;
                        continue;
                    }
                    Some(E9::I(msg)) => {
                        log::debug!("Received MsgChannelFailure");
                        let _: MsgChannelFailure = msg;
                        //let _: ChannelOpenRequest = requests.take()?;
                        //requests.respond(ConnectionResponse::OpenFailure(msg.reason))?;
                        continue;
                    }
                    None => {
                        log::error!("FIXME: unimplemented");
                        continue;
                    }
                }
            }
        }
    }
}
