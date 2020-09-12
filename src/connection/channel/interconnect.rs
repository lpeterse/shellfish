use super::*;
use crate::util::socket::Socket;
use async_std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct Interconnect<S: Socket> {
    s1: ChannelHandle,
    s2: S,
    s1_closed: bool,
    s2_closed: bool,
}

impl<S: Socket> Interconnect<S> {
    pub(crate) fn new(channel: ChannelHandle, socket: S) -> Self {
        Self {
            s1: channel,
            s2: socket,
            s1_closed: false,
            s2_closed: false,
        }
    }
}

impl<S: Socket> Future for Interconnect<S> {
    type Output = Result<(), std::io::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let self_ = Pin::into_inner(self);
        let s1_closed: &mut bool = &mut self_.s1_closed;
        let s2_closed: &mut bool = &mut self_.s2_closed;
        let mut s2: Pin<&mut S> = Pin::new(&mut self_.s2);

        self_.s1.with_state(|s1| {
            if !*s1_closed {
                while !s1.std_rx().is_empty() {
                    match s2.as_mut().poll_write(cx, s1.std_rx().as_ref()) {
                        Poll::Pending => break,
                        Poll::Ready(result) => {
                            let written = result?;
                            if written > 0 {
                                s1.std_rx().consume(written);
                                s1.inner_task_wake = true;
                                continue;
                            }
                        }
                    }
                }
                if s1.reof {
                    match s2.as_mut().poll_close(cx) {
                        Poll::Pending => (),
                        Poll::Ready(result) => {
                            result?;
                            *s1_closed = true;
                            s1.std_rx();
                        }
                    }
                } else {
                    match s2.as_mut().poll_flush(cx) {
                        Poll::Pending => (),
                        Poll::Ready(result) => result?,
                    }
                }
                if s1.reof || s1.rclose {
                    *s1_closed = true;
                }
            }

            while !*s2_closed {
                if s1.std_tx().available() == 0 {
                    s1.std_tx().pushback()
                }
                if s1.std_tx().available() == 0 && s1.std_tx().len() < s1.max_buffer_size as usize {
                    use std::cmp::{max, min};
                    let old = s1.std_tx().capacity();
                    let new = min(max(old * 2, 1024), s1.max_buffer_size as usize);
                    s1.std_tx().increase_capacity(new);
                }
                let rws = s1.rws as usize;
                let buf = s1.std_tx().available_mut();
                let len = std::cmp::min(buf.len(), rws);
                match s2.as_mut().poll_read(cx, &mut buf[..len]) {
                    Poll::Pending => break,
                    Poll::Ready(result) => {
                        let read = result?;
                        s1.inner_task_wake = true;
                        if read > 0 {
                            let _ = s1.std_tx().extend(read);
                        } else {
                            s1.leof = true;
                            s1.lclose = true;
                            *s2_closed = true;
                            break;
                        }
                    }
                }
            }

            if *s1_closed && *s2_closed {
                Poll::Ready(Ok(()))
            } else {
                s1.register_outer_task(cx);
                Poll::Pending
            }
        })
    }
}
