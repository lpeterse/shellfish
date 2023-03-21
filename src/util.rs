use std::{future::Future, pin::Pin};

pub mod buffer;
pub mod cidr;
pub mod codec;
pub mod glob;
pub mod process;
pub mod pty;
pub mod secret;
pub mod socket;
pub mod socks5;

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
