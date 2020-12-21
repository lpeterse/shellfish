use super::Encode;
use sha2::Digest;

use crate::util::check;

/// An `Encoder` is anything that is able to process a sequence of basic encoding operations
/// and assembles them into a result. It is most likely a bytestream assembler, but might also
/// be a hash function or something similar.
///
/// All operations shall return `Optional` instead of panicking. When being inlined the compiler is
/// usually able to generate quite efficient code and can merge subsequent bounds checks or
/// eliminate them completely.
pub trait Encoder: Sized {
    /// Push a `u8` to the encoder state.
    ///
    /// Returns `None` if the encoder had insufficient capacity.
    #[must_use]
    fn push_u8(&mut self, x: u8) -> Option<()>;
    /// Push a `u32` in big-endian representation to the encoder state.
    ///
    /// Returns `None` if the encoder had insufficient capacity.
    #[must_use]
    fn push_u32be(&mut self, x: u32) -> Option<()>;
    ///  Push a `u64` in big-endian representation to the encoder state.
    ///
    /// Returns `None` if the encoder had insufficient capacity.
    #[must_use]
    fn push_u64be(&mut self, x: u64) -> Option<()>;
    /// Push raw bytes to the encoder state.
    ///
    /// Returns `None` if the encoder had insufficient capacity.
    #[must_use]
    fn push_bytes(&mut self, x: &[u8]) -> Option<()>;
}

impl<D: Digest> Encoder for D {
    fn push_u8(&mut self, x: u8) -> Option<()> {
        self.update([x]);
        Some(())
    }
    fn push_u32be(&mut self, x: u32) -> Option<()> {
        self.update(x.to_be_bytes());
        Some(())
    }
    fn push_u64be(&mut self, x: u64) -> Option<()> {
        self.update(x.to_be_bytes());
        Some(())
    }
    fn push_bytes(&mut self, x: &[u8]) -> Option<()> {
        self.update(x);
        Some(())
    }
}

#[derive(Debug)]
pub struct SliceEncoder<'a> {
    pos: usize,
    buf: &'a mut [u8],
}

impl<'a> SliceEncoder<'a> {
    /// Create a new slice encoder from a mutable piece of memory.
    pub fn new(x: &'a mut [u8]) -> Self {
        Self { pos: 0, buf: x }
    }

    /// Encode a given structue into a `Vec<u8>`.
    ///
    /// Panics if the actual encoding size would exceed the calculated `size` as reported by the
    /// structure.
    pub fn encode<E: Encode>(e: &E) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.resize(e.size(), 0);
        let mut enc = SliceEncoder {
            pos: 0,
            buf: &mut vec,
        };
        if Encode::encode(e, &mut enc).is_none() {
            panic!("Calculated size was insufficient for encoding")
        }
        vec
    }

    pub fn encode_into<E: Encode>(e: &E, buf: &'a mut [u8]) {
        if Encode::encode(e, &mut Self::new(buf)).is_none() {
            panic!("Supplied buffer was insufficient for encoding")
        }
    }
}

impl<'a> Encoder for SliceEncoder<'a> {
    #[inline(always)]
    fn push_u8(self: &mut Self, x: u8) -> Option<()> {
        check(self.buf.len() > self.pos)?;
        self.buf[self.pos] = x;
        self.pos += 1;
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

    #[inline(always)]
    fn push_bytes(self: &mut Self, x: &[u8]) -> Option<()> {
        let x: &[u8] = x.as_ref();
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
    fn test_bencoder_debug() {
        let enc = SliceEncoder::new(&mut [][..]);
        assert_eq!("SliceEncoder { pos: 0, buf: [] }", format!("{:?}", enc));
    }

    #[test]
    fn test_bencoder_push_u8() {
        let mut buf = [0; 2];
        let mut enc = SliceEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_u8(23), Some(()));
        assert_eq!(enc.push_u8(47), Some(()));
        assert_eq!([23, 47], buf);
    }

    #[test]
    fn test_bencoder_push_u32be() {
        let mut buf = [0; 8];
        let mut enc = SliceEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_u32be(0x01020304), Some(()));
        assert_eq!(enc.push_u32be(0x05060708), Some(()));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }

    #[test]
    fn test_bencoder_push_u64be() {
        let mut buf = [0; 16];
        let mut enc = SliceEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_u64be(0x0102030405060708), Some(()));
        assert_eq!(enc.push_u64be(0x0909090909090909), Some(()));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9, 9, 9, 9, 9], buf);
    }

    #[test]
    fn test_bencoder_push_bytes() {
        let mut buf = [0; 8];
        let mut enc = SliceEncoder::new(&mut buf[..]);
        assert_eq!(enc.push_bytes(&[1, 2, 3, 4]), Some(()));
        assert_eq!(enc.push_bytes(&[5, 6, 7, 8]), Some(()));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }

    #[test]
    fn test_digest_debug() {
        let enc = SliceEncoder::new(&mut [][..]);
        assert_eq!("SliceEncoder { pos: 0, buf: [] }", format!("{:?}", enc));
    }

    #[test]
    fn test_digest_push_u8() {
        let mut digest = sha2::Sha256::new();
        assert_eq!(digest.push_u8(0x01), Some(()));
        assert_eq!([75, 245, 18, 47, 52, 69, 84, 197], digest.finalize()[..8]);
    }

    #[test]
    fn test_digest_push_u32be() {
        let mut digest = sha2::Sha256::new();
        assert_eq!(digest.push_u32be(0x01020304), Some(()));
        assert_eq!(
            [159, 100, 167, 71, 225, 185, 127, 19],
            digest.finalize()[..8]
        );
    }
    #[test]
    fn test_digest_push_u64be() {
        let mut digest = sha2::Sha256::new();
        assert_eq!(digest.push_u64be(0x0102030405060708), Some(()));
        assert_eq!([102, 132, 13, 218, 21, 78, 138, 17], digest.finalize()[..8]);
    }

    #[test]
    fn test_digest_push_bytes() {
        let mut digest = sha2::Sha256::new();
        assert_eq!(digest.push_bytes(b"1234"), Some(()));
        assert_eq!([3, 172, 103, 66, 22, 243, 225, 92], digest.finalize()[..8]);
    }
}
