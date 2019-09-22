use async_std::io::Write;
use futures::io::AsyncWrite;
use std::pin::*;
use futures::ready;
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

    pub fn reserve(&mut self, len: usize) -> Option<&mut [u8]> {
        assert!(len <= MAX_BUFFER_SIZE);
        if self.available() < len {
            None
        } else {
            let start = self.window.end;
            self.window.end += len;
            let end = self.window.end;
            Some(&mut self.buffer[start .. end])
        }
    }

    pub async fn alloc(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        let available = self.buffer.len() - self.window.end;
        if available < len {
            self.flush().await?;
        }
        let available = self.buffer.len();
        if available < len {
            let mut new_size = available;
            loop {
                new_size *= 2;
                if new_size >= MAX_BUFFER_SIZE {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "max buffer size exhausted",
                    ));
                };
                if new_size >= len {
                    break;
                };
            }
            let mut vec = Vec::with_capacity(new_size);
            vec.resize(new_size, 0);
            self.buffer = vec.into_boxed_slice();
        }
        let start = self.window.end;
        self.window.end += len;
        Ok(&mut self.buffer[start..self.window.end])
    }

    pub async fn flush(&mut self) -> async_std::io::Result<()> {
        self.stream
            .write_all(&self.buffer[..self.window.end])
            .await?;
        self.window.end = 0;
        Ok(())
    }

    pub fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), std::io::Error>> {
        let s = Pin::into_inner(self);
        loop {
            if s.window.len() == 0 {
                return Poll::Ready(Ok(()))
            }
            let buf = &s.buffer[s.window.start .. s.window.end];
            match ready!(Pin::new(&mut s.stream).poll_write(cx, buf)) {
                Err(e) => {
                    return Poll::Ready(Err(e));
                }
                Ok(written) => {
                    s.window.start += written;
                    if s.window.len() == 0 {
                        s.window = Range { start: 0, end: 0 };
                        return Poll::Ready(Ok(()))
                    }
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
