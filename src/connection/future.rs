use super::state::*;
use super::*;

use async_std::future::Future;
use async_std::task::{Context, Poll, Waker};
use std::pin::*;
use std::sync::{Arc, Mutex};

/// The `ConnectionFuture` handles all events related with a single connection.
///
/// The future needs to be constantly polled in order to drive the connection handling. It is
/// supposed to be run as isolated task. The future only resolves on error which also designates
/// the end of the connection's lifetime.
#[derive(Debug)]
pub(crate) struct ConnectionFuture(Arc<Mutex<ConnectionState>>);

impl ConnectionFuture {
    pub fn new(state: &Arc<Mutex<ConnectionState>>) -> Self {
        Self(state.clone())
    }
}

impl Future for ConnectionFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let reference_count = Arc::strong_count(&self.0);
        log::debug!("Reference count: {}", reference_count);
        if reference_count <= 1 {
            return Poll::Ready(());
        }
        let (poll, waker) = {
            let mut x = self.0.lock().unwrap();
            (x.poll(cx).map(|r| x.terminate(r)), x.outer_task_waker())
        };
        // Wake the other task _after_ the Mutex lock has been released.
        let _ = waker.map(Waker::wake);
        poll
    }
}
