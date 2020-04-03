use super::*;
use super::{ConnectionError, ConnectionFuture};

use crate::transport::*;

use async_std::task::Context;

pub(crate) fn poll<T: TransportLayer>(
    x: &mut ConnectionFuture<T>,
    cx: &mut Context,
) -> Poll<Result<(), ConnectionError>> {
    for channel in x.channels.iter() {
        ready!(channel.poll(cx, &mut x.transport))?
    }
    Poll::Pending
}
