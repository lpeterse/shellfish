use super::msg_channel_request::*;
use super::*;
use super::{ConnectionError, ConnectionFuture};

use crate::transport::*;

use futures::task::Context;

pub fn poll<R: Role, T: TransportStream>(
    x: &mut ConnectionFuture<R,T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    for channel in x.channels.iter() {
        // Nothing to do if channel is closing.
        // We're expecting the peer's close message any moment..
        // The channel remove logic is located in the disconnect
        // message handler.
        if channel.is_closing {
            continue;
        }
        match channel.shared {
            TypedState::Session(ref st) => {
                let mut shared = st.lock().unwrap();
                shared.connection_task.register(cx.waker());
                match shared.specific.request {
                    RequestState::Open(ref r) => {
                        let msg = MsgChannelRequest {
                            recipient_channel: channel.remote_channel,
                            want_reply: true,
                            request: r,
                        };
                        ready!(x.transport.poll_send(cx, &msg))?;
                        shared.specific.request = RequestState::Progress;
                    }
                    _ => (),
                }
            }
        }
    }

    Poll::Pending
}
