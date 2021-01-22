use super::*;

use std::future::Future;
use std::task::{ready, Context};
use std::pin::Pin;

pub struct ChannelOpenFuture<C: Channel> {
    phantom: std::marker::PhantomData<C>,
    rx: OpenOutboundRx,
}

impl<C: Channel> ChannelOpenFuture<C> {
    pub(crate) fn new(rx: OpenOutboundRx) -> Self {
        Self {
            phantom: Default::default(),
            rx,
        }
    }
}

impl<C: Channel> Future for ChannelOpenFuture<C> {
    type Output = Result<Result<C, ChannelOpenFailure>, ConnectionError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let self_: &mut Self = Pin::into_inner(self);
        let result = if let Some(r) = ready!(Pin::new(&mut self_.rx).poll(cx)) {
            r.map(|x| x.map(<C as Channel>::new))
        } else {
            Err(ConnectionError::Unknown)
        };
        Poll::Ready(result)
    }
}
