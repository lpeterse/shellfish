use super::{
    ChannelOpenFailure, ChannelOpenFailureReason, ChannelOpenRequest, ConnectionError,
    ConnectionFuture, DisconnectRequest, MsgChannelOpen, Request, Session,
};
use crate::transport::*;
use crate::role::*;
use crate::socket::*;

use futures::ready;
use futures::task::{Context, Poll};

pub fn poll<R: Role, T: Socket>(
    x: &mut ConnectionFuture<R,T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    match ready!(x.request_receiver.poll(cx))? {
        Request::Disconnect(_) => {
            log::debug!("Command::Disconnect");
            let msg = MsgDisconnect::new(Reason::BY_APPLICATION);
            ready!(x.transport.poll_send(cx, &msg))?;
            x.request_receiver.accept();
            x.request_receiver
                .complete(|_: DisconnectRequest| Ok(((), ())))?;
            return Poll::Ready(Ok(()));
        }
        Request::ChannelOpen(r) => {
            log::debug!("Command::ChannelOpenSession");
            match x.channels.free() {
                None => {
                    // In case of local channel shortage, reject the request.
                    x.request_receiver.accept();
                    x.request_receiver.complete(|_: ChannelOpenRequest| {
                        let failure = ChannelOpenFailure {
                            reason: ChannelOpenFailureReason::RESOURCE_SHORTAGE,
                        };
                        Ok((Err(failure), ()))
                    })?;
                }
                Some(local_channel) => {
                    let msg: MsgChannelOpen<Session> = MsgChannelOpen {
                        sender_channel: local_channel,
                        initial_window_size: r.input.initial_window_size,
                        maximum_packet_size: r.input.max_packet_size,
                        channel_type: (),
                    };
                    ready!(x.transport.poll_send(cx, &msg))?;
                    x.request_receiver.accept();
                }
            }
            return Poll::Ready(Ok(()));
        }
    }
}
