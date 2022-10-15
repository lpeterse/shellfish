use std::{pin::Pin, future::Future};

pub mod buffer;
pub mod cidr;
pub mod codec;
pub mod glob;
pub mod socket;
pub mod socks5;
pub mod secret;

pub type ArcError = std::sync::Arc<dyn std::error::Error + Send + Sync + 'static>;
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

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
