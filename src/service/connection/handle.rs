use super::future::*;
use super::state::*;
use super::*;

use crate::transport::DisconnectReason;

use async_std::task::{ready, Context, Poll};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct ConnectionHandle<T: TransportLayer = Transport>(Arc<Mutex<ConnectionState<T>>>);

impl<T: TransportLayer> ConnectionHandle<T> {
    pub fn new(config: &Arc<ConnectionConfig>, transport: T) -> Self {
        let state = ConnectionState::new(config, transport);
        let state = Arc::new(Mutex::new(state));
        let future = ConnectionFuture::new(&state);
        async_std::task::spawn(future);
        Self(state)
    }

    pub fn open<C: Channel>(&mut self, params: C::Open) -> ChannelOpenFuture<C> {
        let mut x = self.0.lock().unwrap();
        x.open_channel(params)
    }

    pub fn request(&mut self, name: String, data: Vec<u8>) {
        let mut x = self.0.lock().unwrap();
        x.request_global(name, data)
    }

    pub fn request_want_reply(&mut self, name: String, data: Vec<u8>) -> ReplyFuture {
        let mut x = self.0.lock().unwrap();
        x.request_global_want_reply(name, data)
    }

    pub fn disconnect(&mut self, reason: DisconnectReason) {
        let mut x = self.0.lock().unwrap();
        x.disconnect(reason)
    }

    pub fn poll_next_request(
        &mut self,
        cx: &mut Context,
    ) -> Poll<Option<Result<ConnectionRequest, ConnectionError>>> {
        let mut x = self.0.lock().unwrap();
        Poll::Ready(Some(ready!(x.poll_next(cx))))
    }
}
