mod requests;
mod transport;
mod channels;

use super::channel::*;
use super::msg_channel_open::*;
use super::*;

use crate::requestable;
use crate::transport::*;

use futures::future::Future;
use futures::ready;
use futures::task::{Context, Poll};
use std::pin::*;

pub struct ConnectionFuture<T> {
    pub future: TransportFuture<T>,
    pub request_sender: requestable::Sender<Connection>,
    pub request_receiver: requestable::Receiver<Connection>,
    pub channels: ChannelMap,
}

impl<T: TransportStream> ConnectionFuture<T> {
    pub fn new(
        transport: Transport<T>,
        request_sender: requestable::Sender<Connection>,
        request_receiver: requestable::Receiver<Connection>,
    ) -> Self {
        Self {
            request_sender,
            request_receiver,
            future: TransportFuture::Ready(transport),
            channels: ChannelMap::new(256),
        }
    }

    pub fn terminate(&mut self, e: ConnectionError) -> Poll<ConnectionError> {
        //self.request_sender.terminate(e); // FIXME
        self.request_receiver.terminate(e);
        self.channels.terminate(e);
        Poll::Ready(e)
    }

    fn poll_events(
        cx: &mut Context,
        t: Transport<T>,
        request_sender: &mut requestable::Sender<Connection>,
        request_receiver: &mut requestable::Receiver<Connection>,
        channels: &mut ChannelMap,
    ) -> Result<Result<Transport<T>, TransportFuture<T>>, ConnectionError> {
        // Poll for incoming messages
        let t = match transport::poll(cx, t, request_receiver, channels)? {
            Ok(t) => t,
            Err(f) => return Ok(Err(f)),
        };
        // Poll for requests issued on the local connection handle
        let t = match requests::poll(cx, t, request_receiver, channels)? {
            Ok(t) => t,
            Err(f) => return Ok(Err(f)),
        };
        // Poll for channel events
        let t = match channels::poll(cx, t, request_receiver, channels)? {
            Ok(t) => t,
            Err(f) => return Ok(Err(f)),
        };
        log::info!("PENDING");
        Ok(Ok(t))
    }
}

impl<T> Future for ConnectionFuture<T>
where
    T: Unpin + TransportStream,
{
    type Output = ConnectionError;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        log::trace!("ConnectionFuture::poll");
        let mut self_ = Pin::into_inner(self);

        loop {
            let transport = match ready!(Pin::new(&mut self_.future).poll(cx)) {
                Ok(transport) => transport,
                Err(e) => return self_.terminate(e.into()),
            };

            match Self::poll_events(
                cx,
                transport,
                &mut self_.request_sender,
                &mut self_.request_receiver,
                &mut self_.channels,
            ) {
                Ok(Err(future)) => {
                    // This means that the transport is busy with something and needs to
                    // be polled in order to be freed. Loop entry does this or returns pending.
                    self_.future = future;
                    continue;
                }
                Ok(Ok(transport)) => {
                    if !transport.flushed() {
                        // All event sources polled and pending, but there is data available to be
                        // sent. Loop entry will poll the transport future one more time
                        // (important for waker registration) and return pending afterwards.
                        self_.future = transport.flush2();
                        continue;
                    } else {
                        // All events sources (commands, channels etc) polled and pending.
                        // The transport is also flushed so there is nothing more to do.
                        // NB: Must not call continue here (or endless loop until error).
                        self_.future = transport.ready();
                        return Poll::Pending;
                    }
                }
                Err(e) => return self_.terminate(e),
            }
        }
    }
}
