use super::*;

/// SSH specific encoder operations.
pub trait SshEncoder: Encoder {
    #[must_use]
    #[inline]
    fn push<T: SshEncode>(&mut self, x: &T) -> Option<()> {
        x.encode(self)
    }
    #[must_use]
    #[inline]
    fn push_bool(&mut self, x: bool) -> Option<()> {
        self.push_u8(if x { 1 } else { 0 })
    }
    #[must_use]
    #[inline]
    fn push_usize(&mut self, x: usize) -> Option<()> {
        crate::util::check(x <= u32::MAX as usize)?;
        self.push_u32be(x as u32)
    }
    #[must_use]
    #[inline]
    fn push_str(&mut self, x: &str) -> Option<()> {
        self.push_bytes(x.as_bytes())
    }
    #[must_use]
    #[inline]
    fn push_str_framed(&mut self, x: &str) -> Option<()> {
        self.push_usize(x.len())?;
        self.push_str(&x)
    }
    #[must_use]
    #[inline]
    fn push_bytes_framed(&mut self, x: &[u8]) -> Option<()> {
        self.push_usize(x.len())?;
        self.push_bytes(&x)
    }
    #[must_use]
    fn push_list<T: SshEncode>(&mut self, xs: &[T]) -> Option<()> {
        let mut size = SizeEncoder::new();
        for x in xs {
            size.push(x)?
        }
        self.push_usize(size.into())?;
        for x in xs {
            self.push(x)?
        }
        Some(())
    }
    #[must_use]
    fn push_name_list<S: AsRef<str>, T: AsRef<[S]>>(&mut self, xs: T) -> Option<()> {
        let xs = xs.as_ref();
        let commas = std::cmp::max(1, xs.len()) - 1;
        let size = commas + xs.iter().map(|x| x.as_ref().len()).sum::<usize>();
        self.push_usize(size)?;
        if let Some((x, ys)) = xs.split_first() {
            self.push_str(x.as_ref())?;
            for y in ys {
                self.push_u8(b',')?;
                self.push_str(y.as_ref())?;
            }
        }
        Some(())
    }
    /// RFC 4251:
    /// "Represents multiple precision integers in two's complement format,
    /// stored as a string, 8 bits per byte, MSB first.  Negative numbers
    /// have the value 1 as the most significant bit of the first byte of
    /// the data partition.  If the most significant bit would be set for
    /// a positive number, the number MUST be preceded by a zero byte.
    /// Unnecessary leading bytes with the value 0 or 255 MUST NOT be
    /// included.  The value zero MUST be stored as a string with zero
    /// bytes of data."
    #[must_use]
    fn push_mpint(&mut self, x: &[u8]) -> Option<()> {
        let mut x = x;
        while let Some(0) = x.get(0) {
            x = &x[1..];
        }
        if let Some(n) = x.get(0) {
            if *n > 127 {
                self.push_usize(x.len() + 1)?;
                self.push_u8(0)?;
            } else {
                self.push_usize(x.len())?;
            }
            self.push_bytes(&x)?;
        } else {
            self.push_usize(0)?;
        }
        Some(())
    }
}

impl SshEncoder for SizeEncoder {}

impl<'a> SshEncoder for RefEncoder<'a> {}

impl<D: sha2::Digest> SshEncoder for D {}
