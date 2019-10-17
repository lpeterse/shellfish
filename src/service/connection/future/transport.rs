use super::msg_channel_close::MsgChannelClose;
use super::msg_channel_data::MsgChannelData;
use super::msg_channel_eof::MsgChannelEof;
use super::msg_channel_extended_data::MsgChannelExtendedData;
use super::msg_channel_failure::*;
use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_channel_request::*;
use super::msg_channel_success::*;
use super::msg_channel_window_adjust::*;
use super::msg_global_request::*;
use super::msg_request_failure::*;
use super::msg_request_success::*;
use super::*;
use super::{ConnectionError, Session};

use crate::transport::*;

use futures::task::{Context, Poll};
use std::sync::{Arc, Mutex};

pub fn poll<R: Role, T: Socket>(
    x: &mut ConnectionFuture<R, T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    ready!(x.transport.poll_receive(cx))?;
    // MSG_CHANNEL_DATA
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelData = msg;
            log::debug!("Received MSG_CHANNEL_DATA ({} bytes)", msg.data.len());
            let channel = x.channels.get(msg.recipient_channel)?;
            channel.decrease_local_window_size(msg.data.len() as u32)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    let written = state.stdout.write(msg.data);
                    assert!(written == msg.data.len());
                    state.outer_waker.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_EXTENDED_DATA
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelExtendedData = msg;
            log::debug!(
                "Received MSG_CHANNEL_EXTENDED_DATA ({} bytes)",
                msg.data.len()
            );
            let channel = x.channels.get(msg.recipient_channel)?;
            channel.decrease_local_window_size(msg.data.len() as u32)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    let written = state.stderr.write(msg.data);
                    assert!(written == msg.data.len());
                    state.outer_waker.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_WINDOW_ADJUST
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelWindowAdjust = msg;
            log::debug!("Received MSG_CHANNEL_WINDOW_ADJUST");
            let channel = x.channels.get(msg.recipient_channel)?;
            channel.increase_remote_window_size(msg.bytes_to_add)?;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_EOF
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelEof = msg;
            log::debug!("Received MSG_CHANNEL_EOF");
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    state.is_remote_eof = true;
                    state.outer_waker.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_CLOSE
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelClose = msg;
            log::debug!("Received MSG_CHANNEL_CLOSE");
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    if !state.is_closed {
                        let msg = MsgChannelClose {
                            recipient_channel: channel.remote_channel(),
                        };
                        ready!(x.transport.poll_send(cx, &msg))?;
                        state.is_closed = true;
                        state.outer_waker.wake();
                    }
                }
            }
            x.channels.remove(msg.recipient_channel)?;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_OPEN
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelOpen<Session> = msg;
            log::debug!("Received MSG_CHANNEL_OPEN");
            todo!();
        }
    }
    // MSG_CHANNEL_OPEN_CONFIRMATION
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelOpenConfirmation<Session> = msg;
            log::debug!("Received MSG_CHANNEL_OPEN_CONFIRMATION");
            let state = x.request_receiver.complete(|r: ChannelOpenRequest| {
                let shared = Arc::new(Mutex::new(Default::default()));
                let state = Channel::new(
                    msg.recipient_channel,
                    r.initial_window_size,
                    r.max_packet_size,
                    msg.sender_channel,
                    msg.initial_window_size,
                    msg.maximum_packet_size,
                    SharedState::Session(shared.clone()),
                );
                let session: Session = Session::new(shared);
                Ok((Ok(session), state))
            })?;
            x.channels.insert(msg.recipient_channel, state)?;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_OPEN_FAILURE
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelOpenFailure = msg;
            log::debug!("Received MSG_CHANNEL_OPEN_FAILURE");
            x.request_receiver.complete(|_: ChannelOpenRequest| {
                let failure = ChannelOpenFailure { reason: msg.reason };
                Ok((Err(failure), ()))
            })?;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_REQUEST
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelRequest<&[u8]> = msg;
            log::debug!("Received MSG_CHANNEL_REQUEST: {}", msg.request);
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    match msg.request {
                        "env" => {
                            let env = BDecoder::decode(msg.specific)
                                .ok_or(TransportError::DecoderError)?;
                            state.add_env(env);
                        }
                        "exit-status" => {
                            let status = BDecoder::decode(msg.specific)
                                .ok_or(TransportError::DecoderError)?;
                            state.set_exit_status(status);
                        }
                        "exit-signal" => {
                            let signal = BDecoder::decode(msg.specific)
                                .ok_or(TransportError::DecoderError)?;
                            state.set_exit_signal(signal);
                        }
                        _ => {
                            if msg.want_reply {
                                let msg = MsgChannelFailure {
                                    recipient_channel: channel.remote_channel(),
                                };
                                ready!(x.transport.poll_send(cx, &msg))?;
                                log::debug!("Sent MSG_CHANNEL_FAILURE");
                            }
                        }
                    }
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_SUCCESS
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            log::debug!("Received MSG_CHANNEL_SUCCESS");
            let _: MsgChannelSuccess = msg;
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    state.request.success()?;
                    state.outer_waker.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_CHANNEL_FAILURE
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelFailure = msg;
            log::debug!("Received MSG_CHANNEL_FAILURE");
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared() {
                SharedState::Session(ref st) => {
                    let mut state = st.lock().unwrap();
                    state.request.failure()?;
                    state.outer_waker.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_GLOBAL_REQUEST
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgGlobalRequest = msg;
            log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
            if msg.want_reply {
                let msg = MsgRequestFailure;
                ready!(x.transport.poll_send(cx, &msg))?;
                log::debug!("Sent MSG_REQUEST_FAILURE");
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // MSG_REQUEST_SUCCESS
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgRequestSuccess = msg;
            log::debug!("Received MSG_REQUEST_SUCCESS");
            todo!();
        }
    }
    // MSG_REQUEST_FAILURE
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgRequestFailure = msg;
            log::debug!("Received MSG_REQUEST_FAILURE");
            todo!();
        }
    }
    // In case the message cannot be decoded, don't consume the message before the transport has
    // sent a MSG_UNIMPLEMENTED for the corresponding packet number.
    ready!(x.transport.poll_send_unimplemented(cx))?;
    x.transport.consume();
    Poll::Ready(Ok(()))
}
