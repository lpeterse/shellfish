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
    // MSG_CHANNEL_OPEN (session)
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelOpen<Session<Client>> = msg;
        log::debug!("Received MSG_CHANNEL_OPEN (session)");
        /*
        if x.channel_open.is_none() {
            let (s, r) = oneshot::channel();
            let o = CO {
                sender_channel: msg.sender_channel,
                initial_window_size: msg.initial_window_size,
                maximum_packet_size: msg.maximum_packet_size,
                reply: XY::Session(r),
            };
            let req = OpenRequest {
                open: msg.channel_type,
                reply: s,
            };
            let tx = x.request_tx.clone();
            let mut future: BoxFuture<()> =
                Box::pin(async move { tx.send(InboundRequest::OpenSession(req)).await });
            ready!(Pin::new(&mut future).poll(cx));
            x.channel_open = Some(o);
            x.transport.consume();
            return Poll::Ready(Ok(()));
        } else {
            return Poll::Ready(Err(ConnectionError::ChannelOpenUnexpected));
        }*/
        todo!()
    }
    // MSG_CHANNEL_OPEN (direct-tcpip)
    if let Some(msg) = x.transport.decode() {
        let _: MsgChannelOpen<DirectTcpIp> = msg;
        log::debug!("Received MSG_CHANNEL_OPEN (direct-tcpip)");
        /*
        if x.channel_open.is_none() {
            let (s, r) = oneshot::channel();
            let o = CO {
                sender_channel: msg.sender_channel,
                initial_window_size: msg.initial_window_size,
                maximum_packet_size: msg.maximum_packet_size,
                reply: XY::DirectTcpIp(r),
            };
            let req = OpenRequest {
                open: msg.channel_type,
                reply: s,
            };
            let tx = x.request_tx.clone();
            let mut future: BoxFuture<()> =
                Box::pin(async move { tx.send(InboundRequest::OpenDirectTcpIp(req)).await });
            ready!(Pin::new(&mut future).poll(cx));
            x.channel_open = Some(o);
            x.transport.consume();
            return Poll::Ready(Ok(()));
        } else {
            return Poll::Ready(Err(ConnectionError::ChannelOpenUnexpected));
        }*/
        todo!()
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
    if let Some(msg) = x.transport.decode() {
        let _: MsgGlobalRequest = msg;
        log::debug!("Received MSG_GLOBAL_REQUEST: {}", msg.name);
        ready!(x.push_global_request(cx, msg.name, msg.data, msg.want_reply));
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_REQUEST_SUCCESS
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgRequestSuccess = msg;
        log::debug!("Received MSG_REQUEST_SUCCESS");
        if let Some(tx) = x.pending_global.pop_front() {
            tx.send(GlobalReply::Success(msg.data.into()));
        } else {
            return Poll::Ready(Err(ConnectionError::GlobalRequestReplyUnexpected));
        }
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // MSG_REQUEST_FAILURE
    if let Some(msg) = x.transport.decode_ref() {
        let _: MsgRequestFailure = msg;
        log::debug!("Received MSG_REQUEST_FAILURE");
        if let Some(tx) = x.pending_global.pop_front() {
            tx.send(GlobalReply::Failure);
        } else {
            return Poll::Ready(Err(ConnectionError::GlobalRequestReplyUnexpected));
        }
        x.transport.consume();
        return Poll::Ready(Ok(()));
    }
    // Otherwise try to send MSG_UNIMPLEMENTED and return error.
    x.transport.send_unimplemented(cx);
    Poll::Ready(Err(TransportError::MessageUnexpected.into()))
}
