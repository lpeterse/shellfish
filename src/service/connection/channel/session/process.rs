use super::*;
use futures::io::AsyncRead;
use futures::task::{Context, Poll};
use std::pin::Pin;

pub struct Process(Session);

impl Process {
    pub(super) fn new(x: Session) -> Self {
        Self(x)
    }

    pub fn stdin<'a>(&'a mut self) -> Stdin<'a> {
        Stdin(self)
    }

    pub fn stdout<'a>(&'a mut self) -> Stdout<'a> {
        Stdout(self)
    }

    pub fn stderr<'a>(&'a mut self) -> Stderr<'a> {
        Stderr(self)
    }
}

impl AsyncRead for Process {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut (self.stdout())).poll_read(cx, buf)
    }
}

pub struct Stdin<'a>(&'a mut Process);

pub struct Stdout<'a>(&'a mut Process);

impl<'a> AsyncRead for Stdout<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut channel = (self.0).0.channel.lock().unwrap();
        if channel.is_closed {
            return Poll::Ready(Ok(0));
        }
        if !channel.specific.stdout.is_empty() {
            // FIXME wake if window resize possible
            let read = channel.specific.stdout.read(buf);
            return Poll::Ready(Ok(read));
        }
        if channel.is_remote_eof {
            return Poll::Ready(Ok(0));
        }
        channel.user_task.register(cx.waker());
        return Poll::Pending;
    }
}

pub struct Stderr<'a>(&'a mut Process);

impl<'a> AsyncRead for Stderr<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut channel = (self.0).0.channel.lock().unwrap();
        if channel.is_closed {
            return Poll::Ready(Ok(0));
        }
        if !channel.specific.stderr.is_empty() {
            // FIXME wake if window resize possible
            let read = channel.specific.stderr.read(buf);
            return Poll::Ready(Ok(read));
        }
        if channel.is_remote_eof {
            return Poll::Ready(Ok(0));
        }
        channel.user_task.register(cx.waker());
        return Poll::Pending;
    }
}
