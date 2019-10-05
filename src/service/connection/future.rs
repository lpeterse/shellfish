mod channels;
mod requests;
mod transport;

use super::channel::*;
use super::msg_channel_open::*;
use super::*;

use crate::transport::*;
use crate::socket::*;

use futures::future::Future;
use futures::ready;
use futures::task::{Context, Poll};
use std::pin::*;

pub struct ConnectionFuture<R: Role, T> {
    pub transport: Transport<R,T>,
    pub request_sender: RequestSender,
    pub request_receiver: RequestReceiver,
    pub channels: ChannelMap,
}

impl<R: Role, T: Socket> ConnectionFuture<R,T> {
    pub fn new(
        transport: Transport<R,T>,
        request_sender: RequestSender,
        request_receiver: RequestReceiver,
    ) -> Self {
        Self {
            transport,
            request_sender,
            request_receiver,
            channels: ChannelMap::new(256),
        }
    }

    fn terminate(&mut self, e: ConnectionError) -> ConnectionError {
        //self.request_sender.terminate(e); // FIXME
        self.request_receiver.terminate(e);
        self.channels.terminate(e);
        e
    }

    fn poll_events(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        loop {
            // Loop over all event sources until none of it makes progress anymore.
            // The transport shall not be flushed, but might be written to. A consolidated flush
            // will be performed afterwards. This is benefecial for networking performance as it
            // allows multiple messages to be sent in a single TCP segment (even with TCP_NODELAY)
            // and impedes traffic analysis.
            let mut made_progress = true;
            while made_progress {
                made_progress = false;
                // Poll for incoming messages
                match transport::poll(self, cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                }
                // Poll for requests issued on the local connection handle
                match requests::poll(self, cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                }
                // Poll for channel events
                match channels::poll(self, cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                }
            }
            // None of the previous actions shall actively flush the transport.
            // If necessary, the transport will be flushed here after all actions have eventually
            // written their output to the transport. It is necessary to loop again as some actions
            // might be pending on output and unblock as soon as buffer space becomes available
            // again. This is somewhat unlikely and will not occur unless the transport is under
            // heavy load, but it is necessary to consider this for correctness or the connection
            // will stop making progress as soon as single notification gets lost.
            if !self.transport.is_flushed() {
                ready!(self.transport.poll_flush(cx))?;
                continue;
            }
            // Being here means all event sources are pending and the transport is flushed.
            // Return pending as there is really nothing to anymore for now.
            return Poll::Pending;
        }
    }
}

impl<R: Role, T> Future for ConnectionFuture<R,T>
where
    T: Unpin + Socket,
{
    type Output = Result<(), ConnectionError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let self_ = Pin::into_inner(self);
        Poll::Ready(ready!(self_.poll_events(cx)).map_err(|e| self_.terminate(e)))
    }
}
