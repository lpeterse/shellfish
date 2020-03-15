use super::*;

use crate::role::*;
use crate::transport::Socket;

use async_std::task::{ready, Context, Poll};

macro_rules! poll_open {
    ($x:expr, $cx:expr, $r:expr, $ty:ty) => {{
        if let Some(id) = $x.channels.free_id() {
            let msg: MsgChannelOpen<$ty> = MsgChannelOpen {
                sender_channel: id,
                initial_window_size: $r.input.initial_window_size,
                maximum_packet_size: $r.input.max_packet_size,
                channel_type: $r.input.specific.clone(),
            };
            ready!($x.transport.poll_send($cx, &msg))?;
            log::debug!("Sent MSG_CHANNEL_OPEN");
            let channel = Box::new(<$ty as Channel>::new_state(id, &$r.input));
            $x.channels.insert(id, channel)?;
            $x.requests.accept();
        } else {
            $x.requests.accept();
            $x.requests
                .resolve::<OpenRequest<$ty>>(Err(ChannelOpenFailureReason::RESOURCE_SHORTAGE))?;
        }
    }};
}

/// Poll for user requests (like channel open etc).
pub(crate) fn poll<R: Role, S: Socket>(
    x: &mut ConnectionFuture<R, S>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    match ready!(x.requests.poll(cx))? {
        Request::OpenSession(r) => poll_open!(x, cx, r, Session),
        Request::OpenDirectTcpIp(r) => poll_open!(x, cx, r, DirectTcpIp),
    }
    Poll::Ready(Ok(()))
}
