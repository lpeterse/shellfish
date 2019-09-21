use async_std::io::Write;
use futures::io::AsyncWrite;

const MIN_BUFFER_SIZE: usize = 1100;
const MAX_BUFFER_SIZE: usize = 35000;

pub struct BufferedSender<S> {
    stream: S,
    buffer: Box<[u8]>,
    write_end: usize,
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
            write_end: 0,
        }
    }

    pub async fn alloc(&mut self, len: usize) -> async_std::io::Result<&mut [u8]> {
        let available = self.buffer.len() - self.write_end;
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
        let start = self.write_end;
        self.write_end += len;
        Ok(&mut self.buffer[start..self.write_end])
    }

    pub async fn flush(&mut self) -> async_std::io::Result<()> {
        self.stream
            .write_all(&self.buffer[..self.write_end])
            .await?;
        self.write_end = 0;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    //use super::*;
}
