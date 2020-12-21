use super::*;
use sha2::Digest;

pub trait SshEncoder: Encoder {
    #[must_use]
    #[inline]
    /// Push something encodable to the encoder state.
    ///
    /// Returns `None` if the encoder had insufficient capacity.
    fn push<T: Encode>(&mut self, x: &T) -> Option<()> {
        x.encode(self)
    }
    #[must_use]
    #[inline]
    fn push_bool(&mut self, x: bool) -> Option<()> {
        if x {
            self.push_u8(1)
        } else {
            self.push_u8(0)
        }
    }
    #[must_use]
    #[inline]
    fn push_usize(&mut self, x: usize) -> Option<()> {
        check(x <= u32::MAX as usize)?;
        self.push_u32be(x as u32)
    }
    #[must_use]
    #[inline]
    fn push_str_framed(&mut self, x: &str) -> Option<()> {
        self.push_usize(x.len())?;
        self.push_bytes(&x.as_bytes())
    }
    #[must_use]
    #[inline]
    fn push_bytes_framed(&mut self, x: &[u8]) -> Option<()> {
        self.push_usize(x.len())?;
        self.push_bytes(&x)
    }
}

impl<D: Digest> SshEncoder for D {}

impl<'a> SshEncoder for SliceEncoder<'a> {}
