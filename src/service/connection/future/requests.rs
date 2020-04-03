use super::*;

use async_std::task::{ready, Context, Poll};

macro_rules! poll_open {
    ($x:expr, $cx:expr, $r:expr, $ty:ty) => {{
        if let Some(id) = $x.channels.free_id() {
            let msg: MsgChannelOpen<$ty> = MsgChannelOpen {
                sender_channel: id,
                initial_window_size: $x.channel_max_buffer_size as u32,
                maximum_packet_size: $x.channel_max_packet_size as u32,
                channel_type: $r.input.specific.clone(),
            };
            ready!($x.transport.poll_send($cx, &msg))?;
            log::debug!("Sent MSG_CHANNEL_OPEN");
            let channel = <$ty as Channel>::new_state($x.channel_max_buffer_size);
            $x.channels.insert(id, Box::new(channel.into()))?;
            $x.request_rx.accept();
        } else {
            $x.request_rx.accept();
            $x.request_rx
                .resolve::<OpenRequest<$ty>>(Err(ChannelOpenFailureReason::RESOURCE_SHORTAGE))?;
        }
    }};
}

/// Poll for user requests (like channel open etc).
pub(crate) fn poll<T: TransportLayer>(
    x: &mut ConnectionFuture<T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    match ready!(x.request_rx.poll(cx))? {
        Request::OpenSession(r) => poll_open!(x, cx, r, Session),
        Request::OpenDirectTcpIp(r) => poll_open!(x, cx, r, DirectTcpIp),
    }
    Poll::Ready(Ok(()))
}
