use async_std::io::Read;
use futures::future::Future;
use futures::io::AsyncRead;
use futures::ready;
use futures::task::{Context, Poll};
use pin_utils::*;
use std::ops::Range;
use std::pin::Pin;

const MIN_BUFFER_SIZE: usize = 1100;
const MAX_BUFFER_SIZE: usize = 35000;

pub struct BufferedReceiver<S> {
    stream: S,
    buffer: Box<[u8]>,
    window: Range<usize>,
}

impl<S: Read + AsyncRead + Unpin> BufferedReceiver<S> {
    unsafe_pinned!(stream: S);

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

    pub async fn fetch(&mut self, len: usize) -> async_std::io::Result<()> {
        while self.window.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "during buffer.fetch()",
                ));
            }
        }
        Ok(())
    }

    async fn fill(&mut self) -> async_std::io::Result<usize> {
        // Case 1: remaining capacity
        if self.window.end < self.buffer.len() {
            // nothing to do
        }
        // Case 2: no remaining capacity right -> memmove
        else if self.window.end >= self.buffer.len() && self.window.start != 0 {
            self.buffer
                .copy_within(self.window.start..self.window.end, 0);
            self.window.end -= self.window.start;
            self.window.start = 0;
        }
        // Case 3: no remainig capacity at all, but smaller MAX_BUFFER_SIZE -> extend
        else if self.buffer.len() < MAX_BUFFER_SIZE {
            println!("RESIZE");
            let len_old = self.buffer.len();
            let len_new = std::cmp::min(len_old * 2, MAX_BUFFER_SIZE);
            let mut vec = Vec::with_capacity(len_new);
            vec.resize(len_new, 0);
            vec[..len_old].copy_from_slice(&self.buffer[self.window.start..self.window.end]);
            println!("RESIZE {} {}", len_new, len_old);
            self.buffer = vec.into_boxed_slice();
            self.window.start = 0;
            self.window.end = len_old;
        }
        // Case 4: no remaining capacity at all, MAX_BUFFER_SIZE reached -> err
        else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "max buffer size exhausted",
            ));
        }
        let read = self
            .stream
            .read(&mut self.buffer[self.window.end..])
            .await?;
        self.window.end += read;
        Ok(read)
    }

    pub async fn read(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        if self.window.len() == 0 {
            if self.fill().await? == 0 {
                return Ok(&mut self.buffer[0..0]);
            }
        }
        if len >= self.window.len() {
            let r = &mut self.buffer[self.window.start..self.window.end];
            self.window.start = 0;
            self.window.end = 0;
            Ok(r)
        } else {
            let r = &mut self.buffer[self.window.start..self.window.start + len];
            self.window.start += len;
            Ok(r)
        }
    }

    pub async fn peek_exact(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        while self.window.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "during buffer.peek_exact()",
                ));
            }
        }
        Ok(&mut self.buffer[self.window.start..][..len])
    }

    pub async fn read_u32be(&mut self) -> async_std::io::Result<u32> {
        let x = self.read_exact(4).await?;
        let mut y = [0; 4];
        y.copy_from_slice(x);
        Ok(u32::from_be_bytes(y))
    }

    pub async fn read_exact(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        while self.window.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "during buffer.read_exact()",
                ));
            }
        }
        if len >= self.window.len() {
            let r = &mut self.buffer[self.window.start..self.window.end];
            self.window.start = 0;
            self.window.end = 0;
            Ok(r)
        } else {
            let r = &mut self.buffer[self.window.start..self.window.start + len];
            self.window.start += len;
            Ok(r)
        }
    }

    pub async fn read_line(&mut self, max_len: usize) -> async_std::io::Result<&[u8]> {
        let mut i = self.window.start;
        loop {
            while i + 2 <= self.window.end {
                if self.buffer[i] == 0x0d && self.buffer[i + 1] == 0x0a {
                    let r = &self.buffer[self.window.start..i];
                    self.window.start = i + 2;
                    return Ok(r);
                }
                i += 1;
            }
            if self.window.len() >= max_len {
                break;
            };
            if self.fill().await? == 0 {
                return Ok(&mut self.buffer[0..0]);
            }
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "max line len exceeded",
        ));
    }

    pub fn poll_fill(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut s = Pin::into_inner(self);
        // Case 1: remaining capacity right >  0
        if s.window.end < s.buffer.len() {
            // nothing to do
        }
        // Case 2: no remaining capacity right -> memmove
        else if s.window.end >= s.buffer.len() && s.window.start != 0 {
            s.buffer.copy_within(s.window.start..s.window.end, 0);
            s.window.end -= s.window.start;
            s.window.start = 0;
        }
        // Case 3: no remainig capacity at all, but smaller MAX_BUFFER_SIZE -> extend
        else if s.buffer.len() < MAX_BUFFER_SIZE {
            println!("RESIZE");
            let len_old = s.buffer.len();
            let len_new = std::cmp::min(len_old * 2, MAX_BUFFER_SIZE);
            let mut vec = Vec::with_capacity(len_new);
            vec.resize(len_new, 0);
            vec[..len_old].copy_from_slice(&s.buffer[s.window.start..s.window.end]);
            println!("RESIZE {} {}", len_new, len_old);
            s.buffer = vec.into_boxed_slice();
            s.window.start = 0;
            s.window.end = len_old;
        }
        // Case 4: no remaining capacity at all, MAX_BUFFER_SIZE reached -> err
        else {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "max buffer size exhausted",
            )));
        }
        // Poll-read the underlying stream. This is always safe as
        // the remaining buffer capacity is recalculated/adapted on every poll.
        Poll::Ready(
            match ready!(Pin::new(&mut s.stream).poll_read(cx, &mut s.buffer[s.window.end..])) {
                Ok(read) => {
                    s.window.end += read;
                    Ok(read)
                }
                Err(e) => Err(e),
            },
        )
    }

    pub fn poll_fetch<'a>(
        mut self: Pin<&'a mut Self>,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<(), std::io::Error>> {
        loop {
            if self.window.len() >= len {
                return Poll::Ready(Ok(()));
            } else {
                match ready!(self.as_mut().poll_fill(cx)) {
                    Ok(_) => continue,
                    Err(e) => return Poll::Ready(Err(e)),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct ChunkedStream(Vec<Vec<u8>>);

    impl AsyncRead for ChunkedStream {
        fn poll_read(
            self: core::pin::Pin<&mut Self>,
            _cx: &mut futures::task::Context,
            buf: &mut [u8],
        ) -> futures::task::Poll<Result<usize, futures::io::Error>> {
            futures::task::Poll::Ready(match self.0.clone().split_first() {
                None => Ok(0),
                Some((head, tail)) => {
                    let mut x: &[u8] = head.as_slice();
                    let read = std::io::Read::read(&mut x, buf)?;
                    if read >= head.len() {
                        self.get_mut().0 = tail.to_vec();
                    } else {
                        let mut v = vec![Vec::from(&head[read..])];
                        v.extend(tail.to_vec());
                        self.get_mut().0 = v;
                    }
                    Ok(read)
                }
            })
        }
    }

    #[test]
    fn test_chunk_reader_read_01() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![]);
            let mut a = [0; 0];
            assert_eq!(r.read(&mut a).await.unwrap(), 0);
        });
    }

    #[test]
    fn test_chunk_reader_read_02() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![]);
            let mut a = [0; 1];
            assert_eq!(r.read(&mut a).await.unwrap(), 0);
        });
    }

    #[test]
    fn test_chunk_reader_read_03() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1, 2]]);
            let mut a = [0; 0];
            assert_eq!(r.read(&mut a).await.unwrap(), 0);
        });
    }

    #[test]
    fn test_chunk_reader_read_04() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1, 2]]);
            let mut a = [0; 1];
            assert_eq!(r.read(&mut a).await.unwrap(), 1);
            assert_eq!(&a[..1], [1]);
        });
    }

    #[test]
    fn test_chunk_reader_read_05() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1, 2]]);
            let mut a = [0; 2];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1, 2]);
        });
    }

    #[test]
    fn test_chunk_reader_read_06() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1, 2]]);
            let mut a = [0; 3];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1, 2]);
        });
    }

    #[test]
    fn test_chunk_reader_read_07() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1, 2, 3]]);
            let mut a = [0; 2];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1, 2]);
            assert_eq!(r.read(&mut a).await.unwrap(), 1);
            assert_eq!(&a[..1], [3]);
        });
    }

    #[test]
    fn test_chunk_reader_read_08() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1, 2], vec![3]]);
            let mut a = [0; 3];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1, 2]);
            assert_eq!(r.read(&mut a).await.unwrap(), 1);
            assert_eq!(&a[..1], [3]);
        });
    }

    #[test]
    fn test_buffer_read_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1, 2], vec![3]]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read(0).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_02() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1, 2], vec![3]]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read(1).await.unwrap(), [1]);
            assert_eq!(b.read(1).await.unwrap(), [2]);
            assert_eq!(b.read(1).await.unwrap(), [3]);
            assert_eq!(b.read(1).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_03() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1, 2], vec![3, 4]]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read(3).await.unwrap(), [1, 2]);
            assert_eq!(b.read(3).await.unwrap(), [3, 4]);
            assert_eq!(b.read(3).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_04() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1, 2], vec![3, 4]]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read(3).await.unwrap(), [1, 2]);
            assert_eq!(b.read(0).await.unwrap(), []);
            assert_eq!(b.read(1).await.unwrap(), [3]);
            assert_eq!(b.read(2).await.unwrap(), [4]);
            assert_eq!(b.read(1).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_exact_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1, 2], vec![3, 4]]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read_exact(3).await.unwrap(), [1, 2, 3]);
            assert!(b.read_exact(3).await.unwrap_err().kind() == std::io::ErrorKind::UnexpectedEof);
        });
    }

    #[test]
    fn test_buffer_line_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![b'A', b'B'], vec![b'\r'], vec![b'\n', b'C']]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read_line(4).await.unwrap(), [b'A', b'B']);
            assert_eq!(b.read(1).await.unwrap(), [b'C']);
        });
    }
}
