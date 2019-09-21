use async_std::io::{Read, Write};
use futures::io::{AsyncRead,AsyncWrite};
use std::ops::Range;
use futures::task::Context;
use futures::task::Poll;

const MIN_BUFFER_SIZE: usize = 1100;
const MAX_BUFFER_SIZE: usize = 35000;

pub struct Buffer<S> {
    stream: S,
    read_buf: Box<[u8]>,
    read_rng: Range<usize>,
    write_buf: Box<[u8]>,
    write_end: usize,
}

impl <S: Read + AsyncRead + Write + AsyncWrite + Unpin> Buffer<S> {
    pub fn new(stream: S) -> Self {
        fn vec() -> Box<[u8]> {
            let mut v = Vec::with_capacity(MIN_BUFFER_SIZE);
            v.resize(MIN_BUFFER_SIZE, 0);
            v.into_boxed_slice()
        }
        Self {
            stream,
            read_buf: vec(),
            read_rng: Range { start: 0, end: 0 },
            write_buf: vec(),
            write_end: 0,
        }
    }

    pub async fn fetch(&mut self, len: usize) -> async_std::io::Result<()> {
        while self.read_rng.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "during buffer.fetch()"))
            }
        }
        Ok(())
    }

    async fn fill(&mut self) -> async_std::io::Result<usize> {
        // Case 1: remaining capacity
        if self.read_rng.end < self.read_buf.len() {
            // nothing to do
        }
        // Case 2: no remaining capacity right -> memmove
        else if self.read_rng.end >= self.read_buf.len() && self.read_rng.start != 0 {
            self.read_buf.copy_within(self.read_rng.start..self.read_rng.end, 0);
            self.read_rng.end -= self.read_rng.start;
            self.read_rng.start = 0;
        }
        // Case 3: no remainig capacity at all, but smaller MAX_BUFFER_SIZE -> extend
        else if self.read_buf.len() < MAX_BUFFER_SIZE {
            println!("RESIZE");
            let len_old = self.read_buf.len();
            let len_new = std::cmp::min(len_old * 2, MAX_BUFFER_SIZE);
            let mut vec = Vec::with_capacity(len_new);
            vec.resize(len_new, 0);
            vec[..len_old].copy_from_slice(&self.read_buf[self.read_rng.start..self.read_rng.end]);
            println!("RESIZE {} {}", len_new, len_old);
            self.read_buf = vec.into_boxed_slice();
            self.read_rng.start = 0;
            self.read_rng.end = len_old;
        }
        // Case 4: no remaining capacity at all, MAX_BUFFER_SIZE reached -> err
        else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "max buffer size exhausted"))
        }
        let read = self.stream.read(&mut self.read_buf[self.read_rng.end..]).await?;
        self.read_rng.end += read;
        Ok(read)
   }

   pub async fn read(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        if self.read_rng.len() == 0 {
            if self.fill().await? == 0 {
                return Ok(&mut self.read_buf[0..0]);
            }
        }
        if len >= self.read_rng.len() {
            let r = &mut self.read_buf[self.read_rng.start .. self.read_rng.end];
            self.read_rng.start = 0;
            self.read_rng.end = 0;
            Ok(r)
        } else {
            let r = &mut self.read_buf[self.read_rng.start .. self.read_rng.start + len];
            self.read_rng.start += len;
            Ok(r)
        }
    }

    pub async fn peek_exact(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        while self.read_rng.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "during buffer.peek_exact()"))
            }
        }
        Ok(&mut self.read_buf[self.read_rng.start ..][.. len])
    }

    pub async fn read_u32be(&mut self) -> async_std::io::Result<u32> {
        let x = self.read_exact(4).await?;
        let mut y = [0;4];
        y.copy_from_slice(x);
        Ok(u32::from_be_bytes(y))
    }

    pub async fn read_exact(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        while self.read_rng.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "during buffer.read_exact()"))
            }
        }
        if len >= self.read_rng.len() {
            let r = &mut self.read_buf[self.read_rng.start .. self.read_rng.end];
            self.read_rng.start = 0;
            self.read_rng.end = 0;
            Ok(r)
        } else {
            let r = &mut self.read_buf[self.read_rng.start .. self.read_rng.start + len];
            self.read_rng.start += len;
            Ok(r)
        }
    }

    pub async fn read_line(&mut self, max_len: usize) -> async_std::io::Result<&[u8]> {
        let mut i = self.read_rng.start;
        
        loop {
            while i + 2 <= self.read_rng.end {
                if self.read_buf[i] == 0x0d && self.read_buf[i+1] == 0x0a {
                    let r = &self.read_buf[self.read_rng.start..i];
                    self.read_rng.start = i + 2;
                    return Ok(r)
                }
                i += 1;
            }
            if self.read_rng.len() >= max_len { break };
            if self.fill().await? == 0 {
                return Ok(&mut self.read_buf[0..0]);
            }
        }
        
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "max line len exceeded"))
    }

    pub async fn alloc(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        let available = self.write_buf.len() - self.write_end;
        if available < len {
            self.flush().await?;
        }
        let available = self.write_buf.len();
        if available < len {
            let mut new_size = available;
            loop {
                new_size *= 2;
                if new_size >= MAX_BUFFER_SIZE {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "max buffer size exhausted"))
                };
                if new_size >= len {
                    break
                };
            }
            let mut vec = Vec::with_capacity(new_size);
            vec.resize(new_size, 0);
            self.write_buf = vec.into_boxed_slice();
        }
        let start = self.write_end;
        self.write_end += len;
        Ok(&mut self.write_buf[start .. self.write_end])
    }

    pub async fn flush(&mut self) -> async_std::io::Result<()> {
        self.stream.write_all(&self.write_buf[.. self.write_end]).await?;
        self.write_end = 0;
        Ok(())
    }
}

pub struct Fill {}

#[cfg(test)]
mod test {
    use super::*;

    struct ChunkedStream (Vec<Vec<u8>>);

    impl AsyncRead for ChunkedStream {
        fn poll_read(
            self: core::pin::Pin<&mut Self>,
            _cx: &mut futures::task::Context,
            buf: &mut [u8]
        ) -> futures::task::Poll<Result<usize, futures::io::Error>> {
            futures::task::Poll::Ready(match self.0.clone().split_first() {
                None => Ok(0),
                Some((head,tail)) => {
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

    impl AsyncWrite for ChunkedStream {
        fn poll_write(self: std::pin::Pin<&mut Self>, _cx: &mut futures::task::Context, _buf: &[u8])
            -> futures::task::Poll<Result<usize, futures::io::Error>> {
            panic!("")
        }

        fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut futures::task::Context)
             -> futures::task::Poll<Result<(), futures::io::Error>> {
            panic!("")
        }

        fn poll_close(self: std::pin::Pin<&mut Self>, _cx: &mut futures::task::Context)
            -> futures::task::Poll<Result<(), futures::io::Error>> {
            panic!("")
        }
    }

    #[test]
    fn test_chunk_reader_read_01() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![]);
            let mut a = [0;0];
            assert_eq!(r.read(&mut a).await.unwrap(), 0);
        });
    }

    #[test]
    fn test_chunk_reader_read_02() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![]);
            let mut a = [0;1];
            assert_eq!(r.read(&mut a).await.unwrap(), 0);
        });
    }

    #[test]
    fn test_chunk_reader_read_03() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1,2]]);
            let mut a = [0;0];
            assert_eq!(r.read(&mut a).await.unwrap(), 0);
        });
    }

    #[test]
    fn test_chunk_reader_read_04() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1,2]]);
            let mut a = [0;1];
            assert_eq!(r.read(&mut a).await.unwrap(), 1);
            assert_eq!(&a[..1], [1]);
        });
    }

    #[test]
    fn test_chunk_reader_read_05() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1,2]]);
            let mut a = [0;2];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1,2]);
        });
    }

    #[test]
    fn test_chunk_reader_read_06() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1,2]]);
            let mut a = [0;3];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1,2]);
        });
    }

    #[test]
    fn test_chunk_reader_read_07() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1,2,3]]);
            let mut a = [0;2];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1,2]);
            assert_eq!(r.read(&mut a).await.unwrap(), 1);
            assert_eq!(&a[..1], [3]);
        });
    }

    #[test]
    fn test_chunk_reader_read_08() {
        async_std::task::block_on(async {
            let mut r = ChunkedStream(vec![vec![1,2], vec![3]]);
            let mut a = [0;3];
            assert_eq!(r.read(&mut a).await.unwrap(), 2);
            assert_eq!(&a[..2], [1,2]);
            assert_eq!(r.read(&mut a).await.unwrap(), 1);
            assert_eq!(&a[..1], [3]);
        });
    }

    #[test]
    fn test_buffer_read_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3]]);
            let mut b = Buffer::new(r);
            assert_eq!(b.read(0).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_02() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3]]);
            let mut b = Buffer::new(r);
            assert_eq!(b.read(1).await.unwrap(), [1]);
            assert_eq!(b.read(1).await.unwrap(), [2]);
            assert_eq!(b.read(1).await.unwrap(), [3]);
            assert_eq!(b.read(1).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_03() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3,4]]);
            let mut b = Buffer::new(r);
            assert_eq!(b.read(3).await.unwrap(), [1,2]);
            assert_eq!(b.read(3).await.unwrap(), [3,4]);
            assert_eq!(b.read(3).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_04() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3,4]]);
            let mut b = Buffer::new(r);
            assert_eq!(b.read(3).await.unwrap(), [1,2]);
            assert_eq!(b.read(0).await.unwrap(), []);
            assert_eq!(b.read(1).await.unwrap(), [3]);
            assert_eq!(b.read(2).await.unwrap(), [4]);
            assert_eq!(b.read(1).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_exact_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3,4]]);
            let mut b = Buffer::new(r);
            assert_eq!(b.read_exact(3).await.unwrap(), [1,2,3]);
            assert!(b.read_exact(3).await.unwrap_err().kind() == std::io::ErrorKind::UnexpectedEof);
        });
    }

    #[test]
    fn test_buffer_line_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![b'A',b'B'], vec![b'\r'], vec![b'\n',b'C']]);
            let mut b = Buffer::new(r);
            assert_eq!(b.read_line(4).await.unwrap(), [b'A',b'B']);
            assert_eq!(b.read(1).await.unwrap(), [b'C']);
        });
    }
}
