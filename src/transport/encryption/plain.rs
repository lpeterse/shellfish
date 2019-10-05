use super::*;

use std::convert::TryInto;

pub struct PlainEncryptionContext {}

impl PlainEncryptionContext {
    const BLOCK_SIZE: usize = 8;

    pub fn new() -> Self {
        Self {}
    }

    pub fn buffer_layout(&self, payload_len: usize) -> PacketLayout {
        PacketLayout::new(payload_len, Self::BLOCK_SIZE, 0)
    }

    pub fn encrypt_packet(&self, _pc: u64, layout: PacketLayout, buf: &mut [u8]) {
        layout.put_len(buf);
        layout.pad_zero(buf);
    }

    pub fn decrypt_len(&self, _pc: u64, len: [u8; 4]) -> usize {
        PacketLayout::PACKET_LEN_SIZE + (u32::from_be_bytes(len) as usize)
    }

    pub fn decrypt_packet<'a>(&self, _pc: u64, buf: &'a mut [u8]) -> Option<&'a [u8]> {
        Some(&buf[PacketLayout::PACKET_LEN_SIZE..])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_buffer_layout_01() {
        let ctx = PlainEncryptionContext::new();
        let layout = ctx.buffer_layout(23);
        assert_eq!(5, layout.payload_range().start);
        assert_eq!(28, layout.payload_range().end);
        assert_eq!(28, layout.padding_range().start);
        assert_eq!(32, layout.padding_range().end);
        assert_eq!(32, layout.buffer_len());
    }

    #[test]
    fn test_encrypt_packet_01() {
        let ctx = PlainEncryptionContext::new();
        let mut actual = [9, 9, 9, 9, 9, 5, 5, 5, 5, 5, 9, 9, 9, 9, 9, 9];
        let expected = [0, 0, 0, 12, 6, 5, 5, 5, 5, 5, 0, 0, 0, 0, 0, 0];
        let layout = ctx.buffer_layout(5);
        ctx.encrypt_packet(0, layout, &mut actual[..]);
        assert_eq!(expected, actual);
    }

}
