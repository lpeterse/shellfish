use async_std::io::Write;
use futures::io::AsyncWrite;
use futures::io::AsyncWriteExt;
use std::pin::*;
use futures::ready;
use futures::future::poll_fn;
use futures::task::{Poll,Context};
use std::ops::Range;

const MIN_BUFFER_SIZE: usize = 1100;
const MAX_BUFFER_SIZE: usize = 35000;

pub struct BufferedSender<S> {
    stream: S,
    buffer: Box<[u8]>,
    window: Range<usize>,
}

impl<S: Write + AsyncWrite + Unpin> BufferedSender<S> {
    pub fn new(stream: S) -> Self {
        fn vec() -> Box<[u8]> {
            let mut v = Vec::with_capacity(MIN_BUFFER_SIZE);
            v.resize(MIN_BUFFER_SIZE, 0);
            v.into_boxed_slice()
        }
        Self {
            stream,
            buffer: vec(),
            window: Range { start: 0, end: 0 },
        }
    }

    pub fn available(&self) -> usize {
        self.buffer.len() - self.window.end
    }

    // FIXME: This function should not exist
    pub async fn reserve(&mut self, len: usize) -> Result<&mut [u8], std::io::Error> {
        poll_fn(|cx| {
            ready!(self.poll_reserve(cx, len))?;
            Poll::Ready(Ok::<(), std::io::Error>(()))
        }).await?;
        let start = self.window.end - len;
        let end = self.window.end;
        Ok(&mut self.buffer[start..end])
    }

    pub async fn flush(&mut self) -> async_std::io::Result<()> {
        self.stream
            .write_all(&self.buffer[..self.window.end])
            .await?;
        self.window.end = 0;
        Ok(())
    }

    pub fn flushed(&self) -> bool {
        self.window.len() == 0
    }

    pub fn poll_flush(&mut self, cx: &mut Context) -> Poll<Result<(), std::io::Error>> {
        loop {
            if self.window.len() == 0 {
                return Poll::Ready(Ok(()))
            }
            let buf = &self.buffer[self.window.start .. self.window.end];
            match ready!(Pin::new(&mut self.stream).poll_write(cx, buf)) {
                Err(e) => {
                    return Poll::Ready(Err(e));
                }
                Ok(written) => {
                    self.window.start += written;
                    if self.window.len() == 0 {
                        self.window = Range { start: 0, end: 0 };
                        return Poll::Ready(Ok(()))
                    }
                    continue;
                }
            }
        }
    }

    pub fn poll_reserve(&mut self, cx: &mut Context, len: usize) -> Poll<Result<&mut [u8], std::io::Error>> {
        assert!(len <= MAX_BUFFER_SIZE);
        // If the available space is insufficient, first try to flush the buffer.
        if self.available() < len {
            ready!(self.poll_flush(cx))?;
        }
        // If the available space is still insufficient, resize the buffer.
        // Copying data is not necessary as the buffer is flushed.
        if self.available() < len {
            let mut vec = Vec::with_capacity(len);
            vec.resize(len, 0);
            self.buffer = vec.into_boxed_slice();
            self.window.start = 0;
            self.window.end = 0;
        }
        // The buffer window (unsent data) is extended to the right by requested len.
        let start = self.window.end;
        self.window.end += len;
        Poll::Ready(Ok(&mut self.buffer[start..self.window.end]))
    }
}

#[cfg(test)]
mod tests {
    //use super::*;
}
