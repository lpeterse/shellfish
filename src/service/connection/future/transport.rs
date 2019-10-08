use super::msg_channel_close::MsgChannelClose;
use super::msg_channel_data::MsgChannelData;
use super::msg_channel_eof::MsgChannelEof;
use super::msg_channel_extended_data::MsgChannelExtendedData;
use super::msg_channel_failure::*;
use super::msg_channel_request::*;
use super::msg_channel_open_confirmation::*;
use super::msg_channel_open_failure::*;
use super::msg_channel_success::*;
use super::msg_channel_window_adjust::*;
use super::msg_global_request::*;
use super::*;
use super::{ConnectionError, Session};

use crate::transport::*;

use futures::task::{Context, Poll};
use std::sync::{Arc, Mutex};

pub fn poll<R: Role, T: Socket>(
    x: &mut ConnectionFuture<R,T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {

    ready!(x.transport.poll_receive(cx))?;
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelData = msg;
            log::debug!("Received MSG_CHANNEL_DATA ({} bytes)", msg.data.len());
            let channel = x.channels.get(msg.recipient_channel)?;
            channel.decrease_local_window_size(msg.data.len())?;
            match channel.shared {
                TypedState::Session(ref st) => {
                    let mut shared = st.lock().unwrap();
                    let written = shared.specific.stdout.write(msg.data);
                    assert!(written == msg.data.len());
                    shared.user_task.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelExtendedData = msg;
            log::debug!("Received MSG_CHANNEL_EXTENDED_DATA ({} bytes)", msg.data.len());
            let channel = x.channels.get(msg.recipient_channel)?;
            channel.decrease_local_window_size(msg.data.len())?;
            match channel.shared {
                TypedState::Session(ref st) => {
                    let mut shared = st.lock().unwrap();
                    let written = shared.specific.stderr.write(msg.data);
                    assert!(written == msg.data.len());
                    shared.user_task.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelEof = msg;
            log::debug!("Received MSG_CHANNEL_EOF");
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared {
                TypedState::Session(ref st) => {
                    let mut shared = st.lock().unwrap();
                    shared.is_remote_eof = true;
                    shared.user_task.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelClose = msg;
            log::debug!("Received MSG_CHANNEL_CLOSE");
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared {
                TypedState::Session(ref st) => {
                    let mut shared = st.lock().unwrap();
                    if !shared.is_closed {
                        let msg = MsgChannelClose {
                            recipient_channel: channel.remote_channel,
                        };
                        ready!(x.transport.poll_send(cx, &msg))?;
                        shared.is_closed = true;
                        shared.user_task.wake();
                    }
                }
            }
            x.channels.remove(msg.recipient_channel);
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgGlobalRequest = msg;
            log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelOpenConfirmation<Session> = msg;
            log::debug!("Received MSG_CHANNEL_OPEN_CONFIRMATION");
            let state = x.request_receiver.complete(|r: ChannelOpenRequest| {
                let shared = Arc::new(Mutex::new(Default::default()));
                let state = ChannelState {
                    is_closing: false,
                    local_channel: msg.recipient_channel,
                    local_window_size: r.initial_window_size,
                    local_max_packet_size: r.max_packet_size,
                    remote_channel: msg.sender_channel,
                    remote_window_size: msg.initial_window_size,
                    remote_max_packet_size: msg.maximum_packet_size,
                    shared: TypedState::Session(shared.clone()),
                };
                let session: Session = Session { channel: shared };
                Ok((Ok(session), state))
            })?;
            x.channels.insert(state)?;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelOpenFailure = msg;
            log::debug!("Received MSG_CHANNEL_OPEN_FAILURE");
            x.request_receiver.complete(|_: ChannelOpenRequest|{
                let failure = ChannelOpenFailure { reason: msg.reason };
                Ok((Err(failure), ()))
            })?;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            log::debug!("Received MSG_CHANNEL_SUCCESS");
            let _: MsgChannelSuccess = msg;
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared {
                TypedState::Session(ref st) => {
                    let mut shared = st.lock().unwrap();
                    shared.specific.request.success()?;
                    shared.user_task.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelFailure = msg;
            log::debug!("Received MSG_CHANNEL_FAILURE");
            let channel = x.channels.get(msg.recipient_channel)?;
            match channel.shared {
                TypedState::Session(ref st) => {
                    let mut shared = st.lock().unwrap();
                    shared.specific.request.failure()?;
                    shared.user_task.wake();
                }
            }
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode_ref() {
        None => (),
        Some(msg) => {
            let _: MsgChannelRequest2 = msg;
            log::debug!("Received MSG_CHANNEL_REQUEST: {}", msg.request);
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    match x.transport.decode() {
        None => (),
        Some(msg) => {
            let _: MsgChannelWindowAdjust = msg;
            log::debug!("Received MSG_CHANNEL_WINDOW_ADJUST");
            let channel = x.channels.get(msg.recipient_channel)?;
            channel.remote_window_size += msg.bytes_to_add;
            x.transport.consume();
            return Poll::Ready(Ok(()));
        }
    }
    // FIXME: This is an error
    log::error!("UNIMPLEMENTED MESSAGE");
    x.transport.consume();
    Poll::Pending
}
