use super::msg_channel_request::*;
use super::*;
use super::{ConnectionError, ConnectionFuture};

use crate::transport::*;

use async_std::task::Context;

pub fn poll<R: Role, T: Socket>(
    x: &mut ConnectionFuture<R,T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    for channel in x.channels.iter() {
        // Nothing to do if channel is closing.
        // We're expecting the peer's close message any moment..
        // The channel remove logic is located in the disconnect
        // message handler.
        if channel.is_closing() {
            continue;
        }
        match channel.shared() {
            SharedState::Session(ref st) => {
                let mut state = st.lock().unwrap();
                //state.inner_waker.register(cx.waker());
                match state.request {
                    RequestState::Open(ref r) => {
                        let msg = MsgChannelRequest {
                            recipient_channel: channel.remote_channel(),
                            request: r.name(),
                            want_reply: true,
                            specific: r,
                        };
                        ready!(x.transport.poll_send(cx, &msg))?;
                        state.request = RequestState::Progress;
                    }
                    _ => (),
                }
            }
        }
    }

    Poll::Pending
}
