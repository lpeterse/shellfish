pub mod buffer;
pub mod cidr;
pub mod codec;
pub mod glob;
pub mod socket;
pub mod socks5;
pub mod tcp;
pub mod secret;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub type ArcError = std::sync::Arc<dyn std::error::Error + Send + Sync + 'static>;
pub type BoxFuture<T> = core::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

/// Takes a `bool` and converts it `Option<()>` to be used as early return point with `?`.
#[inline(always)]
#[must_use]
pub fn check(x: bool) -> Option<()> {
    if x {
        Some(())
    } else {
        None
    }
}

/// TODO: Replace with `std::task::ready` when stable
#[macro_export]
macro_rules! ready {
    ($e:expr) => {
        match $e {
            std::task::Poll::Ready(t) => t,
            std::task::Poll::Pending => {
                return std::task::Poll::Pending;
            }
        }
    };
}

/// TODO: Replace with `std::task::poll_fn` when stable
pub fn poll_fn<T, F>(f: F) -> PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    PollFn { f }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct PollFn<F> {
    f: F,
}

impl<F> Unpin for PollFn<F> {}

impl<F> fmt::Debug for PollFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PollFn").finish()
    }
}

impl<T, F> Future for PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        (&mut self.f)(cx)
    }
}
