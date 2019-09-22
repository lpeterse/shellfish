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

pub struct ForEach<T, E, H, F, O>
where
    E: Unpin + Stream,
    H: Unpin + FnMut(Transport<T>, Either<Token, E::Item>) -> F,
    F: Unpin + Future<Output = Result<Either<Transport<T>, O>, TransportError>>,
{
    transport_or_future: Option<Either<Transport<T>, F>>,
    events: E,
    handler: H,
    future: Option<F>,
    order: bool,
}

impl <T,E,H,F,O> ForEach<T,E,H,F,O>
where
    E: Unpin + Stream,
    H: Unpin + FnMut(Transport<T>, Either<Token, E::Item>) -> F,
    F: Unpin + Future<Output = Result<Either<Transport<T>, O>, TransportError>>,
{
    pub fn new(transport: Transport<T>, events: E, handler: H) -> Self {
        Self {
            transport_or_future: Some(Either::Left(transport)),
            events,
            handler,
            future: None,
            order: false,
        }
    }
}

#[derive(Debug)]
pub enum ForEachResult<O> {
    Quit(O),
    EventStreamExhausted,
    TransportStreamExhausted,
    TransportError(TransportError),
}

impl<T, E, H, F, O> Future for ForEach<T, E, H, F, O>
where
    T: Unpin + TransportStream,
    E: Unpin + Stream + StreamExt,
    H: Unpin + FnMut(Transport<T>, Either<Token, E::Item>) -> F,
    F: Unpin + Future<Output = Result<Either<Transport<T>, O>, TransportError>>,
{
    type Output = ForEachResult<O>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut s = Pin::into_inner(self);
        loop {
            // If present, the handler future must finish before polling
            // any of the streams again. Depending on the result, the fold may
            // exit before any of the streams is exhausted (early-exit).
            match s.transport_or_future {
                None => {
                    return Poll::Ready(ForEachResult::TransportStreamExhausted)
                }
                Some(Either::Right(future)) => match Pin::new(&mut future).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(r) => match r {
                        Ok(Either::Left(transport)) => {
                            s.transport_or_future = Some(Either::Left(transport));
                            s.order = !s.order; // Reverse order
                        }
                        Ok(Either::Right(o)) => return Poll::Ready(ForEachResult::Quit(o)),
                        Err(e) => return Poll::Ready(ForEachResult::TransportError(e)),
                    },
                },
                Some(Either::Left(mut transport)) => {
                    // Poll the transport stream first.
                    if let Poll::Ready(r) = (&mut transport).poll_next_unpin(cx) {
                        //let Some(Either::Left(transport)) = std::mem::replace(&mut s.transport_or_future, None);
                        s.transport_or_future = Some(Either::Right(match r {
                            None => return Poll::Ready(ForEachResult::TransportStreamExhausted),
                            Some(Err(e)) => return Poll::Ready(ForEachResult::TransportError(e)),
                            Some(Ok(token)) => (&mut s.handler)(transport, Either::Left(token)),
                        }));
                        continue;
                    }
                    if let Poll::Ready(r) = Pin::new(&mut s.events).poll_next(cx) {
                        //let transport = std::mem::replace(&mut s.transport_or_future, None).unwrap().left().unwrap();
                        s.transport_or_future = Some(Either::Right(match r {
                            None => return Poll::Ready(ForEachResult::EventStreamExhausted),
                            Some(event) => (&mut s.handler)(transport, Either::Right(event)),
                        }));
                        continue;
                    }
                    return Poll::Pending
                }
            }
            /*
            // Introduce fairness between the streams by changing order of consumption.
            // TODO: Remove code duplication (using a macro?)
            if s.order {
                // Poll the transport stream first.
                if let Poll::Ready(r) = Pin::new(&mut s.transport).poll_next(cx) {
                    s.future = match r {
                        None => return Poll::Ready(ForEachResult::TransportStreamExhausted),
                        Some(Err(e)) => return Poll::Ready(ForEachResult::TransportError(e)),
                        Some(Ok(token)) => (&mut s.handler)(&mut s.transport, Either::Left(token)),
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
                        Some(Err(e)) => return Poll::Ready(ForEachResult::TransportError(e)),
                        Some(Ok(token)) => (&mut s.handler)(&mut s.transport, Either::Left(token)),
                    }
                    .into();
                    continue;
                }
            }*/
            // Getting here means no handler future is pending and none of the streams is ready.
            return Poll::Pending;
        }
    }
}
