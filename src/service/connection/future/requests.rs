use super::{
    ChannelOpenFailureReason, ConnectionError, ConnectionRequest, MsgChannelOpen, Session,
};

use super::{ConnectionFuture, ConnectionResponse};
use crate::transport::*;

use futures::ready;
use futures::task::{Context, Poll};

pub fn poll<T: TransportStream>(
    x: &mut ConnectionFuture<T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    match ready!(x.request_receiver.poll(cx))? {
        ConnectionRequest::Debug(ref msg) => {
            log::debug!("Command::Debug");
            let msg = MsgDebug::new(msg.clone());
            ready!(x.transport.poll_send(cx, &msg))?;
            x.request_receiver.accept()?;
            x.request_receiver.respond(ConnectionResponse::Ok)?;
            return Poll::Ready(Ok(()));
        }
        ConnectionRequest::Disconnect => {
            log::debug!("Command::Disconnect");
            let msg = MsgDisconnect::by_application("".into());
            ready!(x.transport.poll_send(cx, &msg))?;
            x.request_receiver.accept()?;
            x.request_receiver.respond(ConnectionResponse::Ok)?;
            return Poll::Ready(Ok(()));
        }
        ConnectionRequest::ChannelOpen(r) => {
            log::debug!("Command::ChannelOpenSession");
            match x.channels.free() {
                None => {
                    // In case of local channel shortage, reject the request.
                    x.request_receiver.accept()?;
                    x.request_receiver
                        .respond(ChannelOpenFailureReason::RESOURCE_SHORTAGE)?;
                }
                Some(local_channel) => {
                    let msg: MsgChannelOpen<Session> = MsgChannelOpen {
                        sender_channel: local_channel,
                        initial_window_size: r.initial_window_size,
                        maximum_packet_size: r.max_packet_size,
                        channel_type: (),
                    };
                    ready!(x.transport.poll_send(cx, &msg))?;
                    x.request_receiver.accept()?;
                }
            }
            return Poll::Ready(Ok(()));
        }
    }
}
