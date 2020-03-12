use super::*;
use async_std::io::Read;
use async_std::task::{Context, Poll};
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

impl Read for Process {
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

impl<'a> Read for Stdout<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut channel = (self.0).0.state.lock().unwrap();
        if channel.is_closed {
            return Poll::Ready(Ok(0));
        }
        if !channel.stdout.is_empty() {
            // FIXME wake if window resize possible
            let read = channel.stdout.read(buf);
            return Poll::Ready(Ok(read));
        }
        if channel.is_remote_eof {
            return Poll::Ready(Ok(0));
        }
        //channel.outer_waker.register(cx.waker());
        return Poll::Pending;
    }
}

pub struct Stderr<'a>(&'a mut Process);

impl<'a> Read for Stderr<'a> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut channel = (self.0).0.state.lock().unwrap();
        if channel.is_closed {
            return Poll::Ready(Ok(0));
        }
        if !channel.stderr.is_empty() {
            // FIXME wake if window resize possible
            let read = channel.stderr.read(buf);
            return Poll::Ready(Ok(read));
        }
        if channel.is_remote_eof {
            return Poll::Ready(Ok(0));
        }
        //channel.outer_waker.register(cx.waker());
        return Poll::Pending;
    }
}
