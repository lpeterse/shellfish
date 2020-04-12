use super::*;
use crate::buffer::*;

use async_std::task::{ready, Context, Poll};
use std::io::Result;
use std::io::{Error, ErrorKind};
use std::pin::*;

/// The MSG_KEX_INIT message is roughly about 1000 bytes. All other messages
/// are smaller unless the client starts data transfer with larger chunk size
/// for which the buffer automatically adjusts on demand.
const INITIAL_BUFFER_SIZE: usize = 1100;

/// Wraps socket-like data types with send and receive buffer capabilities.
pub struct Buffered<S: Socket> {
    /// Underlying socket/stream.
    socket: S,
    /// Receive buffer.
    rx: Buffer,
    /// Send buffer.
    tx: Buffer,
}

impl<S: Socket> Buffered<S> {
    /// Create a new buffered socket.
    pub fn new(socket: S) -> Self {
        Self {
            socket,
            rx: Buffer::new(INITIAL_BUFFER_SIZE),
            tx: Buffer::new(INITIAL_BUFFER_SIZE),
        }
    }

    /// Check whether the send buffer is empty.
    pub fn flushed(&self) -> bool {
        self.tx.len() == 0
    }

    /// Remove `len` bytes from the receive buffer.
    pub fn consume(&mut self, len: usize) {
        self.rx.consume(len)
    }

    pub async fn fill(&mut self) -> Result<()> {
        poll_fn(move |cx| self.poll_fill(cx)).await
    }

    pub async fn flush(&mut self) -> Result<()> {
        poll_fn(move |cx| self.poll_flush(cx)).await
    }

    /// Extend the buffer by `len` bytes for sending and return it as mutable slice.
    ///
    /// If space is not immediately available it will first try to flush and eventually grow the
    /// send buffer.
    pub fn poll_extend(&mut self, cx: &mut Context, len: usize) -> Poll<Result<&mut [u8]>> {
        // If the available capacity is insufficient, resize the tx.buffer.
        if len > self.tx.capacity() {
            ready!(self.poll_flush(cx))?;
            self.tx.increase_capacity(len)
        }
        // If the available space is still insufficient, try to flush the tx.buffer.
        if len > self.tx.available() {
            ready!(self.poll_flush(cx))?;
        }
        Poll::Ready(Ok(self.tx.extend(len)))
    }

    pub fn poll_fill(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        if self.rx.available() == 0 {
            self.rx.pushback()
        }
        if self.rx.available() == 0 {
            self.rx.increase_capacity(2 * self.rx.capacity());
        }
        // Poll-read the underlying socket. This is always safe as
        // the remaining rx.buffer capacity is recalculated/adapted on every poll.
        let read = ready!(Pin::new(&mut self.socket).poll_read(cx, self.rx.available_mut()))?;
        if read > 0 {
            let _ = self.rx.extend(read);
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "")))
        }
    }

    pub fn poll_fill_exact(&mut self, cx: &mut Context, len: usize) -> Poll<Result<()>> {
        while self.rx.len() < len {
            ready!(self.poll_fill(cx))?
        }
        Poll::Ready(Ok(()))
    }

    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        while self.tx.len() > 0 {
            let written = ready!(Pin::new(&mut self.socket).poll_write(cx, &self.tx.as_ref()))?;
            self.tx.consume(written);
        }
        Poll::Ready(Ok(()))
    }
}

impl<S: Socket> AsRef<[u8]> for Buffered<S> {
    fn as_ref(&self) -> &[u8] {
        self.rx.as_ref()
    }
}

impl<S: Socket> AsMut<[u8]> for Buffered<S> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.rx.as_mut()
    }
}
