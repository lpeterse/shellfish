use super::{
    ChannelMap, ChannelOpenFailureReason, Connection, ConnectionError, ConnectionRequest,
    MsgChannelOpen, Session,
};

use crate::requestable;
use crate::transport::*;

use futures::task::{Context, Poll};

pub fn poll<T: TransportStream>(
    cx: &mut Context,
    mut transport: Transport<T>,
    requests: &mut requestable::Receiver<Connection>,
    channels: &mut ChannelMap,
) -> Result<Result<Transport<T>, TransportFuture<T>>, ConnectionError> {
    match requests.poll(cx) {
        Poll::Pending => return Ok(Ok(transport)),
        Poll::Ready(Err(e)) => return Err(e.into()),
        Poll::Ready(Ok(req)) => match req {
            ConnectionRequest::Debug(ref msg) => {
                log::debug!("Command::Debug");
                let msg = MsgDebug::new(msg.clone());
                match transport.send2(&msg) {
                    Some(()) => {
                        requests.accept()?;
                        return Ok(Ok(transport));
                    }
                    None => {
                        log::debug!("Need to flush first");
                        return Ok(Err(transport.flush2()));
                    }
                }
            }
            ConnectionRequest::Disconnect => {
                log::debug!("Command::Disconnect");
                let msg = MsgDisconnect::by_application("".into());
                match transport.send2(&msg) {
                    Some(()) => {
                        requests.accept()?;
                        return Ok(Err(transport.disconnect()));
                    }
                    None => {
                        log::debug!("Need to flush first");
                        return Ok(Err(transport.flush2()));
                    }
                }
            }
            ConnectionRequest::ChannelOpen(x) => {
                log::debug!("Command::ChannelOpenSession");
                match channels.free_key() {
                    None => {
                        // In case of local channel shortage, reject the request.
                        // It is safe to do nothing if the reply channel was dropped
                        // in the meantime as no resources have been allocated.
                        requests.accept()?;
                        requests.respond(ChannelOpenFailureReason::RESOURCE_SHORTAGE)?;
                        return Ok(Ok(transport));
                    }
                    Some(key) => {
                        let msg: MsgChannelOpen<Session> = MsgChannelOpen {
                            sender_channel: key as u32,
                            initial_window_size: x.initial_window_size,
                            maximum_packet_size: x.max_packet_size,
                            channel_type: (),
                        };
                        match transport.send2(&msg) {
                            Some(()) => {
                                requests.accept()?;
                                return Ok(Ok(transport));
                            }
                            None => {
                                return Ok(Err(transport.flush2()));
                            }
                        }
                    }
                }
            }
        },
    }
}
