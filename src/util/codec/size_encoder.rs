use super::*;

/// A very simple [Encoder] instance that accumulates the required memory requirement
/// for a seqence of encoding operations without actually allocating this memory.
pub struct SizeEncoder(usize);

impl SizeEncoder {
    pub fn new() -> Self {
        Self(0)
    }
}

impl From<SizeEncoder> for usize {
    fn from(x: SizeEncoder) -> usize {
        x.0
    }
}

impl Encoder for SizeEncoder {
    #[inline]
    fn push_u8(&mut self, _: u8) -> Option<()> {
        self.0 += std::mem::size_of::<u8>();
        Some(())
    }
    #[inline]
    fn push_u32be(&mut self, _: u32) -> Option<()> {
        self.0 += std::mem::size_of::<u32>();
        Some(())
    }
    #[inline]
    fn push_u64be(&mut self, _: u64) -> Option<()> {
        self.0 += std::mem::size_of::<u64>();
        Some(())
    }
    #[inline]
    fn push_bytes(&mut self, bytes: &[u8]) -> Option<()> {
        self.0 += bytes.len();
        Some(())
    }
}
