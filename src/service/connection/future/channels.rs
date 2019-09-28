use super::{ChannelMap, Connection, ConnectionError};
use super::msg_channel_request::*;
use super::*;

use crate::requestable;
use crate::transport::*;

use futures::task::{Context};

pub fn poll<T: TransportStream>(
    cx: &mut Context,
    mut transport: Transport<T>,
    requests: &mut requestable::Receiver<Connection>,
    channels: &mut ChannelMap,
) -> Result<Result<Transport<T>, TransportFuture<T>>, ConnectionError> {

    for channel in channels.into_iter() {
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
                        log::info!("REQUEST {:?}", r);
                        let msg = MsgChannelRequest {
                            recipient_channel: channel.remote_channel,
                            want_reply: true,
                            request: r,
                        };
                        match transport.send2(&msg) {
                            Some(()) => {
                                shared.specific.request = RequestState::Progress;
                                log::info!("REQUEST SENT");
                                return Ok(Ok(transport));
                            }
                            None => {
                                return Ok(Err(transport.flush2()));
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    return Ok(Ok(transport))
}
