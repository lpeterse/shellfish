pub mod glob;
pub mod oneshot;

use std::future::Future;

/// Takes a `bool` and converts it `Option<()>` to be used as early return point with `?`.
#[inline(always)]
pub fn assume(x: bool) -> Option<()> {
    if x {
        Some(())
    } else {
        None
    }
}

pub type BoxFuture<T> = core::pin::Pin<Box<dyn Future<Output = T> + Send>>;
