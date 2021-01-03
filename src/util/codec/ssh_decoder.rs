use super::*;

/// SSH specific decoder operations.
pub trait SshDecoder<'a>: Decoder<'a> {
    #[must_use]
    fn take<T: SshDecodeRef<'a>>(&mut self) -> Option<T> {
        T::decode(self)
    }
    #[must_use]
    fn take_bool(&mut self) -> Option<bool> {
        self.take_u8().map(|n| n != 0)
    }
    #[must_use]
    fn take_bytes_framed(&mut self) -> Option<&'a [u8]> {
        let len = self.take_usize()?;
        self.take_bytes(len)
    }
    #[must_use]
    fn take_usize(&mut self) -> Option<usize> {
        // This is safe on all platforms where usize is at least 32 bits.
        // It seems a reasonable assumption that a check can be omitted here.
        Some(self.take_u32be()? as usize)
    }
    #[must_use]
    fn take_list<T: SshDecodeRef<'a>>(&mut self) -> Option<Vec<T>> {
        let bytes = self.take_bytes_framed()?;
        let mut d = RefDecoder::new(bytes);
        let mut v = vec![];
        while d.expect_eoi().is_none() {
            v.push(d.take()?)
        }
        Some(v)
    }
    #[must_use]
    fn take_str(&mut self, len: usize) -> Option<&'a str> {
        let bytes = self.take_bytes(len)?;
        std::str::from_utf8(bytes).ok()
    }
    #[must_use]
    fn take_str_framed(&mut self) -> Option<&'a str> {
        let len = self.take_usize()?;
        self.take_str(len)
    }
    #[must_use]
    fn take_name_list(&mut self) -> Option<std::str::Split<'a, char>> {
        let list = self.take_str_framed()?;
        Some(list.split(','))
    }
    // FIXME
    #[must_use]
    fn take_mpint(&mut self) -> Option<&'a [u8]> {
        self.take_bytes_framed()
    }
    #[must_use]
    fn expect_true(&mut self) -> Option<()> {
        self.take_bool().filter(|y| *y).map(drop)
    }
    #[must_use]
    fn expect_false(&mut self) -> Option<()> {
        self.take_bool().filter(|y| !*y).map(drop)
    }
    #[must_use]
    fn expect_str_framed(&mut self, x: &str) -> Option<()> {
        self.take_str_framed().filter(|y| *y == x).map(drop)
    }
}

impl<'a> SshDecoder<'a> for RefDecoder<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_01() {
        let a = [0, 1 as u8];
        let mut c = RefDecoder::new(&a);

        assert_eq!(c.take_bool(), Some(false));
        assert_eq!(c.take_bool(), Some(true));
        assert_eq!(c.take_bool(), None);
    }

    #[test]
    fn test_take_str_01() {
        let a = "ABCDE".as_bytes();
        let mut c = RefDecoder::new(&a);

        assert_eq!(c.take_str(3), Some("ABC"));
        assert_eq!(c.take_bytes_all(), Some(b"DE".as_ref()));
    }

    #[test]
    fn test_take_str_02() {
        let a = "ABCDE".as_bytes();
        let mut c = RefDecoder::new(&a);

        assert_eq!(c.take_str(5), Some("ABCDE"));
        assert_eq!(c.take_bytes_all(), Some(b"".as_ref()));
    }

    #[test]
    fn test_take_str_03() {
        let a = "ABCDE".as_bytes();
        let mut c = RefDecoder::new(&a);

        assert_eq!(c.take_str(6), None);
    }

    #[test]
    fn test_expect_true_01() {
        let a = [1, 0];
        let mut c = RefDecoder::new(&a);

        assert_eq!(Some(()), c.expect_true());
        assert_eq!(None, c.expect_true());
    }

    #[test]
    fn test_expect_false_01() {
        let a = [0, 1];
        let mut c = RefDecoder::new(&a);

        assert_eq!(Some(()), c.expect_false());
        assert_eq!(None, c.expect_false());
    }
}
