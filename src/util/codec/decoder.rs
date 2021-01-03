/// A state machine that offers basic operations for consumption from an underlying
/// byte input.
pub trait Decoder<'a>: Clone {
    #[must_use]
    fn expect_eoi(&self) -> Option<()>;
    #[must_use]
    fn expect_u8(&mut self, x: u8) -> Option<()>;
    #[must_use]
    fn expect_u32be(&mut self, x: u32) -> Option<()>;
    #[must_use]
    fn expect_bytes(&mut self, bytes: &[u8]) -> Option<()>;
    #[must_use]
    fn take_u8(&mut self) -> Option<u8>;
    #[must_use]
    fn take_u32be(&mut self) -> Option<u32>;
    #[must_use]
    fn take_u64be(&mut self) -> Option<u64>;
    #[must_use]
    fn take_bytes(&mut self, len: usize) -> Option<&'a [u8]>;
    #[must_use]
    fn take_bytes_all(&mut self) -> Option<&'a [u8]>;
    #[must_use]
    fn take_bytes_into(&mut self, buf: &mut [u8]) -> Option<()>;
    #[must_use]
    fn take_bytes_while<F: FnMut(u8) -> bool + Sized>(&mut self, pred: F) -> Option<&'a [u8]>;
}
