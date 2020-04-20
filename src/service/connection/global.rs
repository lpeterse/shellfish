use super::*;

use async_std::future::Future;
use async_std::task::*;
use core::pin::*;

#[derive(Debug)]
pub struct GlobalRequest {
    pub(crate) name: String,
    pub(crate) data: Vec<u8>,
    pub(crate) reply: Option<oneshot::Sender<Result<Vec<u8>, ConnectionError>>>,
}

impl GlobalRequest {
    pub(crate) fn new(name: String, data: Vec<u8>) -> Self {
        Self {
            name,
            data,
            reply: None,
        }
    }

    pub(crate) fn new_want_reply(name: String, data: Vec<u8>) -> (Self, ReplyFuture) {
        let (tx, rx) = oneshot::channel();
        let mut self_ = Self::new(name, data);
        self_.reply = Some(tx);
        (self_, ReplyFuture(rx))
    }

    pub fn accept(self, data: Vec<u8>) {
        let mut self_ = self;
        if let Some(reply) = self_.reply.take() {
            reply.send(Ok(data))
        }
    }

    pub fn reject(self) {
        drop(self)
    }
}

impl GlobalRequest {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }
}

#[derive(Debug)]
pub struct ReplyFuture(oneshot::Receiver<Result<Vec<u8>, ConnectionError>>);

impl Future for ReplyFuture {
    type Output = Result<Option<Vec<u8>>, ConnectionError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.as_mut().0)
            .poll(cx)
            .map(|x| x.map(|y| y.map(Some)).unwrap_or(Ok(None)))
    }
}
