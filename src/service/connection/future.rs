use super::state::*;
use super::*;

use async_std::future::Future;
use async_std::task::{ready, Context, Poll};
use std::pin::*;
use std::sync::{Arc, Mutex};

/// The `ConnectionFuture` handles all events related with a single connection.
///
/// The future needs to be constantly polled in order to drive the connection handling. It is
/// supposed to be run as isolated task. The future only resolves on error which also designates
/// the end of the connection's lifetime.
#[derive(Debug)]
pub(crate) struct ConnectionFuture<T: TransportLayer>(Arc<Mutex<ConnectionState<T>>>);

impl<T: TransportLayer> ConnectionFuture<T> {
    pub fn new(state: &Arc<Mutex<ConnectionState<T>>>) -> Self {
        Self(state.clone())
    }
}

impl<T: TransportLayer> Future for ConnectionFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        log::debug!("Poll");
        let mut x = Pin::into_inner(self).0.lock().unwrap();
        if let Err(e) = ready!(x.poll(cx)) {
            log::debug!("Connection failed with {:?}", e);
            x.terminate(e);
        };
        Poll::Ready(())
    }
}
