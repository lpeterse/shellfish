/// A state machine that is able to process a sequence of basic encoding operations
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

impl<D: sha2::Digest> Encoder for D {
    #[inline]
    fn push_u8(&mut self, x: u8) -> Option<()> {
        self.update([x]);
        Some(())
    }
    #[inline]
    fn push_u32be(&mut self, x: u32) -> Option<()> {
        self.update(x.to_be_bytes());
        Some(())
    }
    #[inline]
    fn push_u64be(&mut self, x: u64) -> Option<()> {
        self.update(x.to_be_bytes());
        Some(())
    }
    #[inline]
    fn push_bytes(&mut self, x: &[u8]) -> Option<()> {
        self.update(x);
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::digest::*;

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
