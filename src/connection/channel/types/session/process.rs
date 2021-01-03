use super::*;
use crate::core::Role;

use async_std::io::Read;
use async_std::stream::Stream;
use async_std::task::{Context, Poll};
use std::pin::Pin;

pub struct Process<R: Role>(pub (crate) Session<R>);

#[derive(Debug, Clone)]
pub enum ProcessEvent {
    //Data,
    //Exit(()),
}

impl<R: Role> Process<R> {
    //pub(super) fn new(x: Session<R>) -> Self {
    //    Self(x)
    //}

    /*
    pub fn stdin<'a>(&'a mut self) -> Stdin<'a, R> {
        Stdin(self)
    }

    pub fn stdout<'a>(&'a mut self) -> Stdout<'a, R> {
        Stdout(self)
    }

    pub fn stderr<'a>(&'a mut self) -> Stderr<'a, R> {
        Stderr(self)
    }

    pub fn eof(&mut self) {}
    */

    //pub fn kill(&mut self, _signal: Signal) {}
}

impl<R: Role> Stream for Process<R> {
    type Item = Result<ProcessEvent, ConnectionError>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
        /*
        let mut state = ((self.0).state)
            .0
            .lock()
            .map_err(|_| ConnectionError::Unknown)?;
        state.outer_task = None;

        if let Some(exit) = state.exit.take() {
            return Poll::Ready(Some(Ok(ProcessEvent::Exit(exit))));
        }
        if let Some(done) = state.inner_done.take() {
            Err(done)?
        }*/

        Poll::Pending
    }
}

//pub struct Stdin<'a, R: Role>(&'a mut Process<R>);

pub struct Stdout<'a, R: Role>(&'a mut Process<R>);

impl<'a, R: Role> Read for Stdout<'a, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        _buf: &mut [u8],
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

pub struct Stderr<'a, R: Role>(&'a mut Process<R>);

impl<'a, R: Role> Read for Stderr<'a, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        _buf: &mut [u8],
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
