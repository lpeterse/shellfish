use super::*;
use crate::transport::PACKET_MIN_LEN;
use crate::transport::PADDING_MIN_LEN;

#[derive(Debug)]
pub struct PlainContext;

impl PlainContext {
    pub const BLOCK_LEN: usize = 8;
    pub const MAC_LEN: usize = 0;

    pub fn new() -> Self {
        Self
    }

    pub fn encrypt(&self, _pc: u64, _buf: &mut [u8]) -> Result<(), TransportError> {
        Ok(())
    }

    pub fn decrypt<'a>(&self, _pc: u64, _buf: &'a mut [u8]) -> Result<(), TransportError> {
        Ok(())
    }

    pub fn decrypt_len(&self, _pc: u64, len: [u8; 4]) -> Result<usize, TransportError> {
        Ok(u32::from_be_bytes(len) as usize)
    }

    pub fn padding_len(&self, payload_len: usize) -> usize {
        let l = 4 + 1 + payload_len;
        let mut p = Self::BLOCK_LEN - (l % Self::BLOCK_LEN);
        if p < PADDING_MIN_LEN {
            p += Self::BLOCK_LEN
        };
        while p + l < PACKET_MIN_LEN {
            p += Self::BLOCK_LEN
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_context_encrypt_01() {
        let ctx = PlainContext::new();
        let mut buf = [1];
        ctx.encrypt(0, &mut buf).unwrap();
        assert_eq!(buf, [1]);
    }

    #[test]
    fn plain_context_decrypt_len_01() {
        let ctx = PlainContext::new();
        assert_eq!(ctx.decrypt_len(0, [0, 0, 0, 23]).unwrap(), 23);
    }

    #[test]
    fn plain_context_padding_len_01() {
        let ctx = PlainContext::new();
        assert_eq!(ctx.padding_len(0), 11);
        assert_eq!(ctx.padding_len(1), 10);
        assert_eq!(ctx.padding_len(2), 9);
        assert_eq!(ctx.padding_len(3), 8);
        assert_eq!(ctx.padding_len(4), 7);
        assert_eq!(ctx.padding_len(5), 6);
        assert_eq!(ctx.padding_len(6), 5);
        assert_eq!(ctx.padding_len(7), 4);
        assert_eq!(ctx.padding_len(8), 11);
        assert_eq!(ctx.padding_len(9), 10);
        assert_eq!(ctx.padding_len(10), 9);
    }
}
