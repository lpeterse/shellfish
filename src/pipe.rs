mod ring_buffer;

pub use self::ring_buffer::*;

use futures::io::{AsyncRead, AsyncWrite};
use futures::task::{AtomicWaker, Context, Poll};
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

pub struct Pipe {
    buffer: RingBuffer,
    is_broken: bool,
    is_closed: bool,
    reader: AtomicWaker,
    writer: AtomicWaker,
}

impl Pipe {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            buffer: RingBuffer::new(max_capacity),
            is_broken: false,
            is_closed: false,
            reader: AtomicWaker::new(),
            writer: AtomicWaker::new(),
        }
    }

    pub fn close(&mut self) {
        self.is_closed = true
    }

    pub fn is_broken(&self) -> bool {
        self.is_broken
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    pub fn split(self) -> (PipeReader, PipeWriter) {
        let x = Arc::new(Mutex::new(self));
        (PipeReader(x.clone()), PipeWriter(x))
    }
}

pub struct PipeReader(Arc<Mutex<Pipe>>);

impl Drop for PipeReader {
    fn drop(&mut self) {
        let mut stream = self.0.lock().unwrap();
        stream.is_broken = true;
        stream.writer.wake();
    }
}

impl AsyncRead for PipeReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut stream = self.0.lock().unwrap();
        stream.reader.register(cx.waker());
        if stream.buffer.is_empty() {
            if stream.is_closed() {
                return Poll::Ready(Ok(0));
            } else if stream.is_broken() {
                return Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "")));
            } else {
                return Poll::Pending;
            }
        }
        let read = stream.buffer.read(buf);
        stream.writer.wake();
        Poll::Ready(Ok(read))
    }
}

pub struct PipeWriter(Arc<Mutex<Pipe>>);

impl Drop for PipeWriter {
    fn drop(&mut self) {
        let mut stream = self.0.lock().unwrap();
        stream.is_broken = true;
        stream.reader.wake();
    }
}

impl AsyncWrite for PipeWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut stream = self.0.lock().unwrap();
        stream.writer.register(cx.waker());
        if stream.is_broken() {
            return Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "")));
        }
        if stream.buffer.is_full() {
            stream.reader.wake();
            return Poll::Pending;
        }
        // Do not wake the reader! `poll_flush` does this!
        let written = stream.buffer.write(buf);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        let stream = self.0.lock().unwrap();
        if stream.is_broken() {
            return Poll::Ready(Err(Error::new(ErrorKind::BrokenPipe, "")));
        }
        if stream.buffer.is_empty() {
            return Poll::Ready(Ok(()))
        }
        stream.writer.register(cx.waker());
        stream.reader.wake();
        Poll::Pending
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Error>> {
        let mut stream = self.0.lock().unwrap();
        stream.close();
        stream.reader.wake();
        Poll::Ready(Ok(()))
    }
}
