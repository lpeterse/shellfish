mod channels;
mod requests;
mod transport;

use super::channel::*;
use super::*;

use crate::transport::{DisconnectReason, TransportError};

use async_std::future::Future;
use async_std::task::{ready, Context, Poll};
use std::pin::*;

/// The `ConnectionFuture` handles all events related with a single connection.
///
/// The future needs to be constantly polled in order to drive the connection handling. It is
/// supposed to be run as isolated task. The future only resolves on error which also designates
/// the end of the connection's lifetime.
pub(crate) struct ConnectionFuture<R: Role, S: Socket> {
    close: oneshot::Receiver<DisconnectReason>,
    channels: Channels,
    requests: RequestReceiver,
    transport: Transport<R, S>,
}

impl<R: Role, T: Socket> ConnectionFuture<R, T> {
    pub fn new<C: ConnectionConfig>(
        config: &C,
        close: oneshot::Receiver<DisconnectReason>,
        requests: RequestReceiver,
        transport: Transport<R, T>,
    ) -> Self {
        Self {
            close,
            channels: Channels::new(config.channel_max_count()),
            requests,
            transport,
        }
    }

    fn poll_events(&mut self, cx: &mut Context) -> Poll<ConnectionError> {
        loop {
            // Loop over all event sources until none of it makes progress anymore.
            // The transport shall not be flushed, but might be written to. A consolidated flush
            // will be performed afterwards. This is benefecial for networking performance as it
            // allows multiple messages to be sent in a single TCP segment (even with TCP_NODELAY)
            // and impedes traffic analysis.
            let mut made_progress = true;
            while made_progress {
                made_progress = false;
                // Poll for local connection close
                match Pin::new(&mut self.close).poll(cx) {
                    Poll::Pending => (),
                    Poll::Ready(reason) => {
                        let reason = reason.unwrap_or_default();
                        self.transport.poll_send_disconnect(cx, reason);
                        return Poll::Ready(TransportError::DisconnectByUs(reason).into());
                    }
                }
                // Poll for incoming messages
                match transport::poll(self, cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                }
                // Poll for requests issued on the local connection handle
                match requests::poll(self, cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                }
                // Poll for channel events
                match channels::poll(self, cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => made_progress = true,
                    Poll::Ready(Err(e)) => return Poll::Ready(e),
                }
            }
            // None of the previous actions shall actively flush the transport.
            // If necessary, the transport will be flushed here after all actions have eventually
            // written their output to the transport. It is necessary to loop again as some actions
            // might be pending on output and unblock as soon as buffer space becomes available
            // again. This is somewhat unlikely and will not occur unless the transport is under
            // heavy load, but it is necessary to consider this for correctness or the connection
            // will stop making progress as soon as a single notification gets lost.
            if !self.transport.flushed() {
                if let Err(e) = ready!(self.transport.poll_flush(cx)) {
                    return Poll::Ready(e.into());
                }
                continue;
            }
            // Being here means all event sources are pending and the transport is flushed.
            // Return pending as there is really nothing to do anymore for now.
            return Poll::Pending;
        }
    }

    /// Deliver a `ConnectionError` to all dependant users of this this connections (tasks waiting
    /// on connection requests or channel I/O).
    /// 
    /// This shall be the last thing to happen and has great similarity with `Drop` except that
    /// it distributes an error.
    fn terminate(&mut self, e: ConnectionError) {
        self.requests.terminate(e);
        self.channels.terminate(e);
    }
}

impl<R: Role, T> Future for ConnectionFuture<R, T>
where
    T: Unpin + Socket,
{
    type Output = ConnectionError;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let self_ = Pin::into_inner(self);
        let e = ready!(self_.poll_events(cx));
        self_.terminate(e);
        Poll::Ready(e)
    }
}
