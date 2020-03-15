use super::*;

use crate::transport::TransportError;

use async_std::task::{Context, Poll};
use std::sync::{Arc, Mutex};

pub(crate) fn poll<R: Role, T: Socket>(
    x: &mut ConnectionFuture<R, T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    ready!(x.transport.poll_receive(cx))?;
    // MSG_CHANNEL_DATA
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelData = msg;
        log::debug!("Received MSG_CHANNEL_DATA ({} bytes)", msg.data.len());
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_data(msg.data)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_EXTENDED_DATA
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelExtendedData = msg;
        log::debug!(
            "Received MSG_CHANNEL_EXTENDED_DATA ({} bytes)",
            msg.data.len()
        );
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_extended_data(msg.data_type_code, msg.data)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_WINDOW_ADJUST
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelWindowAdjust = msg;
        log::debug!("Received MSG_CHANNEL_WINDOW_ADJUST");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_window_adjust(msg.bytes_to_add)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_EOF
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelEof = msg;
        log::debug!("Received MSG_CHANNEL_EOF");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_eof()?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_CLOSE
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelClose = msg;
        log::debug!("Received MSG_CHANNEL_CLOSE");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_close()?;
        ready!(x.transport.poll_send(cx, &msg))?; // FIXME
        x.channels.remove(msg.recipient_channel)?; // FIXME
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_OPEN
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelOpen<Session> = msg;
        log::debug!("Received MSG_CHANNEL_OPEN");
        todo!();
    }
    // MSG_CHANNEL_OPEN_CONFIRMATION
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelOpenConfirmation = msg;
        log::debug!("Received MSG_CHANNEL_OPEN_CONFIRMATION");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_open_confirmation(
            msg.sender_channel,
            msg.initial_window_size,
            msg.maximum_packet_size,
        )?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_OPEN_FAILURE
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelOpenFailure = msg;
        log::debug!("Received MSG_CHANNEL_OPEN_FAILURE");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_open_failure(msg.reason)?;
        x.channels.remove(msg.recipient_channel);
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_REQUEST
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelRequest<&[u8]> = msg;
        log::debug!("Received MSG_CHANNEL_REQUEST: {}", msg.request);
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.request(msg.specific)?;
        /*
        match channel.shared() {
            SharedState::Session(ref st) => {
                let mut state = st.lock().unwrap();
                match msg.request {
                    "env" => {
                        let env = BDecoder::decode(msg.specific)
                            .ok_or(TransportError::DecoderError)?;
                        //state.add_env(env); FIXME
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
        }*/
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_SUCCESS
    if let Some(msg) = x.transport.decode() {
        log::debug!("Received MSG_CHANNEL_SUCCESS");
        let _: MsgChannelSuccess = msg;
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.success()?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_FAILURE
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelFailure = msg;
        log::debug!("Received MSG_CHANNEL_FAILURE");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.fail()?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_GLOBAL_REQUEST
    if let Some(msg) = x.transport.decode_ref() {
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
    // MSG_REQUEST_SUCCESS
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgRequestSuccess = msg;
        log::debug!("Received MSG_REQUEST_SUCCESS");
        todo!(); // FIXME
    }
    // MSG_REQUEST_FAILURE
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgRequestFailure = msg;
        log::debug!("Received MSG_REQUEST_FAILURE");
        todo!(); // FIXME
    }
    // In case the message cannot be decoded, don't consume the message before the transport has
    // sent a MSG_UNIMPLEMENTED for the corresponding packet number.
    x.transport.send_unimplemented(cx);
    Poll::Ready(Err(TransportError::MessageUnexpected.into()))
}
