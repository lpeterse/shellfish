use super::*;
use crate::transport::PACKET_LEN_BYTES;
use crate::transport::PACKET_MAX_LEN;
use crate::transport::PACKET_MIN_LEN;
use crate::transport::PADDING_MIN_LEN;
use crate::util::check;

#[derive(Debug)]
pub struct PlainContext {}

impl PlainContext {
    pub const BLOCK_LEN: usize = 8;
    pub const MAC_LEN: usize = 0;

    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt(&self, _pc: u64, _buf: &mut [u8]) -> Result<(), TransportError> {
        Ok(())
    }

    pub fn decrypt<'a>(&self, _pc: u64, buf: &'a mut [u8]) -> Result<(), TransportError> {
        Ok(())
    }

    pub fn decrypt_len(&self, _pc: u64, len: [u8; 4]) -> Option<usize> {
        Some(u32::from_be_bytes(len) as usize)
        //let len = PACKET_LEN_BYTES + (u32::from_be_bytes(len) as usize);
        //check(len <= PACKET_MAX_LEN)?;
        //Some(len)
    }

    pub fn padding_len(payload_len: usize) -> usize {
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
    fn test_encrypt_01() {
        let ctx = PlainContext::new();
        let mut buf = [1];
        ctx.encrypt(0, &mut buf).unwrap();
        assert_eq!(buf, [1]);
    }

    /* FIXME
    #[test]
    fn test_decrypt_len_01() {
        let ctx = PlainContext::new();
        assert_eq!(Some(27), ctx.decrypt_len(0, [0, 0, 0, 23]));
    }

    #[test]
    fn test_decrypt_len_02() {
        let ctx = PlainContext::new();
        assert_eq!(None, ctx.decrypt_len(0, [0, 0, 255, 0]));
    }
    */
}
