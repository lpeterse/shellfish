use super::*;

use async_std::task::{Context, Poll};

pub(crate) fn poll<T: TransportLayer>(
    x: &mut ConnectionFuture<T>,
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
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_OPEN
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelOpen<Session> = msg;
        log::debug!("Received MSG_CHANNEL_OPEN");
        // todo!()
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
        x.channels.remove(msg.recipient_channel)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_REQUEST
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgChannelRequest<&[u8]> = msg;
        log::debug!("Received MSG_CHANNEL_REQUEST: {}", msg.request);
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_request(msg.specific)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_SUCCESS
    if let Some(msg) = x.transport.decode() {
        log::debug!("Received MSG_CHANNEL_SUCCESS");
        let _: MsgChannelSuccess = msg;
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_success()?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_CHANNEL_FAILURE
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelFailure = msg;
        log::debug!("Received MSG_CHANNEL_FAILURE");
        let channel = x.channels.get(msg.recipient_channel)?;
        channel.push_failure()?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_GLOBAL_REQUEST
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgGlobalRequest = msg;
        log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
        x.global_request_sink
            .push_request(msg.want_reply, msg.data)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_REQUEST_SUCCESS
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgRequestSuccess = msg;
        log::debug!("Received MSG_REQUEST_SUCCESS");
        x.global_request_source.push_success(msg.data)?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_REQUEST_FAILURE
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgRequestFailure = msg;
        log::debug!("Received MSG_REQUEST_FAILURE");
        x.global_request_source.push_failure()?;
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // Otherwise try to send MSG_UNIMPLEMENTED and return error.
    x.transport.send_unimplemented(cx);
    Poll::Ready(Err(TransportError::MessageUnexpected.into()))
}
