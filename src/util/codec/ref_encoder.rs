use super::*;
use crate::util::check;

/// A cursor into a mutable buffer that implements [Encoder].
#[derive(Debug)]
pub struct RefEncoder<'a> {
    pos: usize,
    buf: &'a mut [u8],
}

impl<'a> RefEncoder<'a> {
    /// Create a new slice encoder from a mutable piece of memory.
    pub fn new(x: &'a mut [u8]) -> Self {
        Self { pos: 0, buf: x }
    }

    pub fn is_full(&self) -> bool {
        self.pos >= self.buf.len()
    }
}

impl<'a> Encoder for RefEncoder<'a> {
    #[inline(always)]
    fn push_u8(self: &mut Self, x: u8) -> Option<()> {
        check(self.buf.len() > self.pos)?;
        self.buf[self.pos] = x;
        self.pos += std::mem::size_of::<u8>();
        Some(())
    }

    #[inline(always)]
    fn push_u32be(self: &mut Self, x: u32) -> Option<()> {
        check(self.buf.len() >= self.pos + std::mem::size_of::<u32>())?;
        self.buf[self.pos + 0] = ((x >> 24) & 0xff) as u8;
        self.buf[self.pos + 1] = ((x >> 16) & 0xff) as u8;
        self.buf[self.pos + 2] = ((x >> 8) & 0xff) as u8;
        self.buf[self.pos + 3] = ((x >> 0) & 0xff) as u8;
        self.pos += std::mem::size_of::<u32>();
        Some(())
    }

    #[inline(always)]
    fn push_u64be(self: &mut Self, x: u64) -> Option<()> {
        check(self.buf.len() >= self.pos + std::mem::size_of::<u64>())?;
        self.buf[self.pos + 0] = ((x >> 56) & 0xff) as u8;
        self.buf[self.pos + 1] = ((x >> 48) & 0xff) as u8;
        self.buf[self.pos + 2] = ((x >> 40) & 0xff) as u8;
        self.buf[self.pos + 3] = ((x >> 32) & 0xff) as u8;
        self.buf[self.pos + 4] = ((x >> 24) & 0xff) as u8;
        self.buf[self.pos + 5] = ((x >> 16) & 0xff) as u8;
        self.buf[self.pos + 6] = ((x >> 8) & 0xff) as u8;
        self.buf[self.pos + 7] = ((x >> 0) & 0xff) as u8;
        self.pos += std::mem::size_of::<u64>();
        Some(())
    }

    #[inline]
    fn push_bytes(self: &mut Self, x: &[u8]) -> Option<()> {
        check(self.buf.len() >= self.pos + x.len())?;
        self.buf[self.pos..][..x.len()].copy_from_slice(x);
        self.pos += x.len();
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_u8() {
        let mut buf = [0; 2];
        let mut enc = RefEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_u8(23), Some(()));
        assert_eq!(enc.push_u8(47), Some(()));
        assert_eq!([23, 47], buf);
    }

    #[test]
    fn test_push_u32be() {
        let mut buf = [0; 8];
        let mut enc = RefEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_u32be(0x01020304), Some(()));
        assert_eq!(enc.push_u32be(0x05060708), Some(()));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }

    #[test]
    fn test_push_u64be() {
        let mut buf = [0; 16];
        let mut enc = RefEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_u64be(0x0102030405060708), Some(()));
        assert_eq!(enc.push_u64be(0x0909090909090909), Some(()));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9, 9, 9, 9, 9], buf);
    }

    #[test]
    fn test_push_bytes() {
        let mut buf = [0; 8];
        let mut enc = RefEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_bytes(&[1, 2, 3, 4]), Some(()));
        assert_eq!(enc.push_bytes(&[5, 6, 7, 8]), Some(()));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }
}
