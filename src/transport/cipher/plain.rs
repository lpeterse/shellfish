use super::*;
use crate::util::assume;

pub struct PlainContext {}

impl PlainContext {
    pub const AAD: bool = false;
    pub const BLOCK_LEN: usize = 8;
    pub const MAC_LEN: usize = 0;

    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt(&self, _pc: u64, _buf: &mut [u8]) {
        // Nothing to do
    }

    pub fn decrypt<'a>(&self, _pc: u64, buf: &'a mut [u8]) -> Option<usize> {
        assume(buf.len() > Packet::<()>::PACKET_LEN_LEN)?;
        Some(buf.len() - Packet::<()>::PACKET_LEN_LEN)
    }

    pub fn decrypt_len(&self, _pc: u64, len: [u8; 4]) -> Option<usize> {
        let len = Packet::<()>::PACKET_LEN_LEN + (u32::from_be_bytes(len) as usize);
        assume(len <= Packet::<()>::MAX_PACKET_LEN)?;
        Some(len)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encrypt_01() {
        let ctx = PlainContext::new();
        let mut buf = [1];
        ctx.encrypt(0, &mut buf);
        assert_eq!(buf, [1]);
    }

    #[test]
    fn test_decrypt_01() {
        let ctx = PlainContext::new();
        let mut buf = [0, 0, 0, 1, 23];
        assert_eq!(Some(1), ctx.decrypt(0, &mut buf));
        assert_eq!([0, 0, 0, 1, 23], buf);
    }

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
}
