use super::*;

use async_std::future::Future;
use async_std::task::*;
use core::pin::*;

#[derive(Debug)]
pub struct GlobalRequest {
    pub(crate) name: String,
    pub(crate) data: Vec<u8>,
    pub(crate) reply: Option<oneshot::Sender<Option<Vec<u8>>>>,
}

impl GlobalRequest {
    pub fn new(name: String, data: Vec<u8>) -> Self {
        Self {
            name,
            data,
            reply: None,
        }
    }

    pub fn new_want_reply(name: String, data: Vec<u8>) -> (Self, GlobalReply) {
        let (tx, rx) = oneshot::channel();
        let mut self_ = Self::new(name, data);
        self_.reply = Some(tx);
        (self_, GlobalReply(rx))
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
pub struct GlobalReply(oneshot::Receiver<Option<Vec<u8>>>);

impl Future for GlobalReply {
    type Output = Result<Option<Vec<u8>>, ConnectionError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.as_mut().0)
            .poll(cx)
            .map(|x| x.ok_or(ConnectionError::Terminated))
    }
}
