use super::*;
use async_std::io::Read;
use async_std::stream::Stream;
use async_std::task::{Context, Poll};
use std::pin::Pin;

pub struct Process(Session);

#[derive(Debug, Clone)]
pub enum ProcessEvent {
    Data,
    Exit(Exit),
}

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

    pub fn eof(&mut self) {}

    pub fn kill(&mut self, signal: Signal) {}
}

impl Stream for Process {
    type Item = Result<ProcessEvent, ConnectionError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut state = ((self.0).0).0.lock().map_err(|_| ConnectionError::Terminated)?;
        state.outer_task = None;

        if let Some(exit) = state.exit.take() {
            return Poll::Ready(Some(Ok(ProcessEvent::Exit(exit))))
        }
        if let Some(done) = state.inner_done.take() {
            Err(done)?
        }

        Poll::Pending
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
        /*
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut channel = ((self.0).0).0.lock().unwrap();
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
        */
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
        /*
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut channel = ((self.0).0).0.lock().unwrap();
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
        */
        return Poll::Pending;
    }
}
