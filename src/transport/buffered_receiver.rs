use async_std::io::{Read};
use futures::io::{AsyncRead};
use std::ops::Range;

const MIN_BUFFER_SIZE: usize = 1100;
const MAX_BUFFER_SIZE: usize = 35000;

pub struct BufferedReceiver<S> {
    stream: S,
    buffer: Box<[u8]>,
    range: Range<usize>,
}

impl <S: Read + AsyncRead + Unpin> BufferedReceiver<S> {
    pub fn new(stream: S) -> Self {
        fn vec() -> Box<[u8]> {
            let mut v = Vec::with_capacity(MIN_BUFFER_SIZE);
            v.resize(MIN_BUFFER_SIZE, 0);
            v.into_boxed_slice()
        }
        Self {
            stream,
            buffer: vec(),
            range: Range { start: 0, end: 0 },
        }
    }

    pub async fn fetch(&mut self, len: usize) -> async_std::io::Result<()> {
        while self.range.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "during buffer.fetch()"))
            }
        }
        Ok(())
    }

    async fn fill(&mut self) -> async_std::io::Result<usize> {
        // Case 1: remaining capacity
        if self.range.end < self.buffer.len() {
            // nothing to do
        }
        // Case 2: no remaining capacity right -> memmove
        else if self.range.end >= self.buffer.len() && self.range.start != 0 {
            self.buffer.copy_within(self.range.start..self.range.end, 0);
            self.range.end -= self.range.start;
            self.range.start = 0;
        }
        // Case 3: no remainig capacity at all, but smaller MAX_BUFFER_SIZE -> extend
        else if self.buffer.len() < MAX_BUFFER_SIZE {
            println!("RESIZE");
            let len_old = self.buffer.len();
            let len_new = std::cmp::min(len_old * 2, MAX_BUFFER_SIZE);
            let mut vec = Vec::with_capacity(len_new);
            vec.resize(len_new, 0);
            vec[..len_old].copy_from_slice(&self.buffer[self.range.start..self.range.end]);
            println!("RESIZE {} {}", len_new, len_old);
            self.buffer = vec.into_boxed_slice();
            self.range.start = 0;
            self.range.end = len_old;
        }
        // Case 4: no remaining capacity at all, MAX_BUFFER_SIZE reached -> err
        else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "max buffer size exhausted"))
        }
        let read = self.stream.read(&mut self.buffer[self.range.end..]).await?;
        self.range.end += read;
        Ok(read)
   }

   pub async fn read(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        if self.range.len() == 0 {
            if self.fill().await? == 0 {
                return Ok(&mut self.buffer[0..0]);
            }
        }
        if len >= self.range.len() {
            let r = &mut self.buffer[self.range.start .. self.range.end];
            self.range.start = 0;
            self.range.end = 0;
            Ok(r)
        } else {
            let r = &mut self.buffer[self.range.start .. self.range.start + len];
            self.range.start += len;
            Ok(r)
        }
    }

    pub async fn peek_exact(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        while self.range.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "during buffer.peek_exact()"))
            }
        }
        Ok(&mut self.buffer[self.range.start ..][.. len])
    }

    pub async fn read_u32be(&mut self) -> async_std::io::Result<u32> {
        let x = self.read_exact(4).await?;
        let mut y = [0;4];
        y.copy_from_slice(x);
        Ok(u32::from_be_bytes(y))
    }

    pub async fn read_exact(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        while self.range.len() < len {
            if self.fill().await? == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "during buffer.read_exact()"))
            }
        }
        if len >= self.range.len() {
            let r = &mut self.buffer[self.range.start .. self.range.end];
            self.range.start = 0;
            self.range.end = 0;
            Ok(r)
        } else {
            let r = &mut self.buffer[self.range.start .. self.range.start + len];
            self.range.start += len;
            Ok(r)
        }
    }

    pub async fn read_line(&mut self, max_len: usize) -> async_std::io::Result<&[u8]> {
        let mut i = self.range.start;
        
        loop {
            while i + 2 <= self.range.end {
                if self.buffer[i] == 0x0d && self.buffer[i+1] == 0x0a {
                    let r = &self.buffer[self.range.start..i];
                    self.range.start = i + 2;
                    return Ok(r)
                }
                i += 1;
            }
            if self.range.len() >= max_len { break };
            if self.fill().await? == 0 {
                return Ok(&mut self.buffer[0..0]);
            }
        }
        
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "max line len exceeded"))
    }
}

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
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read(0).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_02() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3]]);
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
            let r = ChunkedStream(vec![vec![1,2], vec![3,4]]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read(3).await.unwrap(), [1,2]);
            assert_eq!(b.read(3).await.unwrap(), [3,4]);
            assert_eq!(b.read(3).await.unwrap(), []);
        });
    }

    #[test]
    fn test_buffer_read_04() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![1,2], vec![3,4]]);
            let mut b = BufferedReceiver::new(r);
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
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read_exact(3).await.unwrap(), [1,2,3]);
            assert!(b.read_exact(3).await.unwrap_err().kind() == std::io::ErrorKind::UnexpectedEof);
        });
    }

    #[test]
    fn test_buffer_line_01() {
        async_std::task::block_on(async {
            let r = ChunkedStream(vec![vec![b'A',b'B'], vec![b'\r'], vec![b'\n',b'C']]);
            let mut b = BufferedReceiver::new(r);
            assert_eq!(b.read_line(4).await.unwrap(), [b'A',b'B']);
            assert_eq!(b.read(1).await.unwrap(), [b'C']);
        });
    }
}
