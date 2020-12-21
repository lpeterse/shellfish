use super::DecodeRef;
use crate::util::*;

use std::convert::TryInto;

pub trait Decoder<'a>: Clone {
    fn take_u8(&mut self) -> Option<u8>;
    fn take_u32be(&mut self) -> Option<u32>;
    fn take_u64be(&mut self) -> Option<u64>;
    fn take_bool(&mut self) -> Option<bool>;
    fn take_str(&mut self, len: usize) -> Option<&'a str>;
    fn take_bytes(&mut self, len: usize) -> Option<&'a [u8]>;
    fn take_bytes_all(&mut self) -> Option<&'a [u8]>;
    fn take_bytes_into(&mut self, buf: &mut [u8]) -> Option<()>;
    fn take_bytes_while<F: FnMut(u8) -> bool + Sized>(&mut self, pred: F) -> Option<&'a [u8]>;

    fn expect_eoi(&self) -> Option<()>;
    fn expect_u8(&mut self, x: u8) -> Option<()>;
    fn expect_u32be(&mut self, x: u32) -> Option<()>;
    fn expect_true(&mut self) -> Option<()>;
    fn expect_false(&mut self) -> Option<()>;
    fn expect_bytes<T: AsRef<[u8]>>(&mut self, bytes: &T) -> Option<()>;
}

/// The `SliceDecoder` is just a shrinking slice of input.
///
/// The state of the decoder is undefined after it failed unless a specific decoder function states
/// something else (no backtracking by default).
#[derive(Copy, Clone, Debug)]
pub struct SliceDecoder<'a>(&'a [u8]);

impl<'a> SliceDecoder<'a> {
    /// Create a new `SliceDecoder`.
    pub fn new(x: &'a [u8]) -> Self {
        Self(x)
    }

    /// Try to decode the given input as `T`.
    ///
    /// All bytes of input must be consumed or the decoding will fail.
    pub fn decode<T: DecodeRef<'a>>(x: &'a [u8]) -> Option<T> {
        let mut d = SliceDecoder(x);
        let r = T::decode(&mut d)?;
        d.expect_eoi()?;
        Some(r)
    }

    /// Try to decode the given input as `T` (as a prefix of the input).
    ///
    /// Decoding will not fail even if not all of the input has been consumed.
    pub fn decode_prefix<T: DecodeRef<'a>>(x: &'a [u8]) -> Option<T> {
        let mut d = SliceDecoder(x);
        T::decode(&mut d)
    }
}

impl<'a> Decoder<'a> for SliceDecoder<'a> {
    fn expect_eoi(&self) -> Option<()> {
        check(self.0.is_empty())
    }

    fn take_u8(self: &mut Self) -> Option<u8> {
        let (n, tail) = self.0.split_first()?;
        self.0 = tail;
        Some(*n)
    }

    fn take_u32be(self: &mut Self) -> Option<u32> {
        check(self.0.len() >= 4)?;
        let (head, tail) = self.0.split_at(4);
        let n = u32::from_be_bytes(head.try_into().ok()?);
        self.0 = tail;
        Some(n)
    }

    fn take_u64be(&mut self) -> Option<u64> {
        check(self.0.len() >= 8)?;
        let (head, tail) = self.0.split_at(8);
        let n = u64::from_be_bytes(head.try_into().ok()?);
        self.0 = tail;
        Some(n)
    }

    fn take_bool(&mut self) -> Option<bool> {
        self.take_u8().map(|n| n != 0)
    }

    fn take_str(&mut self, len: usize) -> Option<&'a str> {
        check(self.0.len() >= len)?;
        let (head, tail) = self.0.split_at(len);
        let s = std::str::from_utf8(head).ok()?;
        self.0 = tail;
        Some(s)
    }

    fn take_bytes(self: &mut Self, len: usize) -> Option<&'a [u8]> {
        check(self.0.len() >= len)?;
        let (s, tail) = self.0.split_at(len);
        self.0 = tail;
        Some(s)
    }

    fn take_bytes_into(self: &mut Self, dst: &mut [u8]) -> Option<()> {
        let s = self.take_bytes(dst.len())?;
        dst.copy_from_slice(s);
        Some(())
    }

    fn take_bytes_all(self: &mut Self) -> Option<&'a [u8]> {
        let s = self.0;
        self.0 = b"";
        Some(s)
    }

    fn take_bytes_while<F>(self: &mut Self, mut pred: F) -> Option<&'a [u8]>
    where
        F: FnMut(u8) -> bool + Sized,
    {
        let mut len = 0;
        for i in self.0 {
            if pred(*i) {
                len += 1;
                continue;
            }
            break;
        }
        self.take_bytes(len)
    }

    fn expect_u8(&mut self, x: u8) -> Option<()> {
        self.take_u8().filter(|y| *y == x).map(drop)
    }

    fn expect_u32be(&mut self, x: u32) -> Option<()> {
        self.take_u32be().filter(|y| *y == x).map(drop)
    }

    fn expect_true(&mut self) -> Option<()> {
        self.take_bool().filter(|y| *y).map(drop)
    }

    fn expect_false(&mut self) -> Option<()> {
        self.take_bool().filter(|y| !*y).map(drop)
    }

    fn expect_bytes<T: AsRef<[u8]>>(self: &mut Self, bytes: &T) -> Option<()> {
        self.take_bytes(bytes.as_ref().len())
            .filter(|x| *x == bytes.as_ref())
            .map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expect_eoi_01() {
        let a = [];
        let c = SliceDecoder::new(&a);

        assert_eq!(c.expect_eoi(), Some(()));
    }

    #[test]
    fn test_expect_eoi_02() {
        let a = [1];
        let c = SliceDecoder::new(&a);

        assert_eq!(c.expect_eoi(), None);
    }

    #[test]
    fn test_take_u8_01() {
        let a = [0, 1, 2, 3 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_u8(), Some(0));
        assert_eq!(c.take_u8(), Some(1));
        assert_eq!(c.take_u8(), Some(2));
        assert_eq!(c.take_u8(), Some(3));
        assert_eq!(c.take_u8(), None);
    }

    #[test]
    fn test_take_u32be_01() {
        let a = [1, 2, 3, 4, 5 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_u32be(), Some(0x01020304));
        assert_eq!(c.take_u32be(), None);
    }

    #[test]
    fn test_take_u64be_01() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_u64be(), Some(0x0102030405060708));
        assert_eq!(c.take_u64be(), None);
    }

    #[test]
    fn test_bool_01() {
        let a = [0, 1 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_bool(), Some(false));
        assert_eq!(c.take_bool(), Some(true));
        assert_eq!(c.take_bool(), None);
    }

    #[test]
    fn test_take_str_01() {
        let a = "ABCDE".as_bytes();
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_str(3), Some("ABC"));
        assert_eq!(c.take_bytes_all(), Some(b"DE".as_ref()));
    }

    #[test]
    fn test_take_str_02() {
        let a = "ABCDE".as_bytes();
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_str(5), Some("ABCDE"));
        assert_eq!(c.take_bytes_all(), Some(b"".as_ref()));
    }

    #[test]
    fn test_take_str_03() {
        let a = "ABCDE".as_bytes();
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_str(6), None);
    }

    #[test]
    fn test_take_bytes_01() {
        let a = [1, 2, 3, 4, 5 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_bytes(3), Some(&[1, 2, 3 as u8][..]));
        assert_eq!(c.take_bytes_all(), Some([4, 5].as_ref()));
    }

    #[test]
    fn test_take_bytes_02() {
        let a = [1, 2, 3, 4, 5 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_bytes(5), Some(&[1, 2, 3, 4, 5 as u8][..]));
        assert_eq!(c.take_bytes_all(), Some([].as_ref()));
    }

    #[test]
    fn test_take_bytes_03() {
        let a = [1, 2, 3, 4, 5 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_bytes(6), None);
    }

    #[test]
    fn test_all_01() {
        let a = [1, 2, 3, 4, 5 as u8];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(c.take_bytes_all(), Some(&[1, 2, 3, 4, 5 as u8][..]));
        assert_eq!(c.0, &[]);
    }

    #[test]
    fn test_into_01() {
        let a = [1, 2, 3, 4, 5, 6 as u8];
        let mut b = [0; 5];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(Some(()), c.take_bytes_into(&mut b));
        assert_eq!(&[1, 2, 3, 4, 5 as u8][..], b);
        assert_eq!(c.take_bytes_all(), Some([6u8].as_ref()));
    }

    #[test]
    fn test_into_02() {
        let a = [1, 2, 3, 4, 5, 6 as u8];
        let mut b = [0; 7];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(None, c.take_bytes_into(&mut b));
    }

    #[test]
    fn test_expect_true_01() {
        let a = [1, 0];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(Some(()), c.expect_true());
        assert_eq!(None, c.expect_true());
    }

    #[test]
    fn test_expect_false_01() {
        let a = [0, 1];
        let mut c = SliceDecoder::new(&a);

        assert_eq!(Some(()), c.expect_false());
        assert_eq!(None, c.expect_false());
    }
}
