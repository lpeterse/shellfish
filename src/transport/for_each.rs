use super::*;

use async_std::net::TcpStream;
use futures::future::Either;
use futures::future::Future;
use futures::stream::{Stream, StreamExt};
use futures::task::Context;
use futures::task::Poll;
use std::marker::Unpin;
use std::option::Option;
use std::pin::Pin;

pub struct ForEach<E, H, F, O>
where
    E: Unpin + Stream,
    H: Unpin + FnMut(&mut Transport<TcpStream>, Either<Token, E::Item>) -> F,
    F: Unpin + Future<Output = Option<O>>,
{
    transport: Transport<TcpStream>,
    events: E,
    handler: H,
    future: Option<F>,
    order: bool,
}

impl <E,H,F,O> ForEach<E,H,F,O>
where
    E: Unpin + Stream,
    H: Unpin + FnMut(&mut Transport<TcpStream>, Either<Token, E::Item>) -> F,
    F: Unpin + Future<Output = Option<O>>,
{
    pub fn new(transport: Transport<TcpStream>, events: E, handler: H) -> Self {
        Self {
            transport,
            events,
            handler,
            future: None,
            order: false,
        }
    }
}

pub enum ForEachResult<O> {
    Quit(O),
    EventStreamExhausted,
    TransportStreamExhausted,
}

impl<E, H, F, O> Future for ForEach<E, H, F, O>
where
    E: Unpin + Stream + StreamExt,
    H: Unpin + FnMut(&mut Transport<TcpStream>, Either<Token, E::Item>) -> F,
    F: Unpin + Future<Output = Option<O>>,
{
    type Output = ForEachResult<O>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = Pin::into_inner(self);
        loop {
            // If present, the handler future must finish before polling
            // any of the streams again. Depending on the result, the fold may
            // exit before any of the streams is exhausted (early-exit).
            match &mut s.future {
                None => (),
                Some(x) => match Pin::new(x).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(r) => match r {
                        None => {
                            s.future = None;
                            s.order = !s.order; // Reverse order
                        }
                        Some(r) => return Poll::Ready(ForEachResult::Quit(r)),
                    },
                },
            }
            // Introduce fairness between the streams by changing order of consumption.
            // TODO: Remove code duplication (using a macro?)
            if s.order {
                // Poll the transport stream first.
                if let Poll::Ready(r) = Pin::new(&mut s.transport).poll_next(cx) {
                    s.future = match r {
                        None => return Poll::Ready(ForEachResult::TransportStreamExhausted),
                        Some(token) => (&mut s.handler)(&mut s.transport, Either::Left(token)),
                    }
                    .into();
                    continue;
                }
                if let Poll::Ready(r) = Pin::new(&mut s.events).poll_next(cx) {
                    s.future = match r {
                        None => return Poll::Ready(ForEachResult::EventStreamExhausted),
                        Some(event) => (&mut s.handler)(&mut s.transport, Either::Right(event)),
                    }
                    .into();
                    continue;
                }
            } else {
                // Poll the event stream first.
                if let Poll::Ready(r) = Pin::new(&mut s.events).poll_next(cx) {
                    s.future = match r {
                        None => return Poll::Ready(ForEachResult::EventStreamExhausted),
                        Some(event) => (&mut s.handler)(&mut s.transport, Either::Right(event)),
                    }
                    .into();
                    continue;
                }
                if let Poll::Ready(r) = Pin::new(&mut s.transport).poll_next(cx) {
                    s.future = match r {
                        None => return Poll::Ready(ForEachResult::TransportStreamExhausted),
                        Some(token) => (&mut s.handler)(&mut s.transport, Either::Left(token)),
                    }
                    .into();
                    continue;
                }
            }
            // Getting here means no handler future is pending and none of the streams is ready.
            return Poll::Pending;
        }
    }
}
