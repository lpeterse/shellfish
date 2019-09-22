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

    pub fn decrypt_buffer_size(&self, _pc: u64, len: &[u8]) -> Option<usize> {
        let len: [u8;4] = len.try_into().ok()?;
        Some(PacketLayout::PACKET_LEN_SIZE + (u32::from_be_bytes(len) as usize))
    }

    pub fn decrypt_len(&self, _pc: u64, len: [u8;4]) -> usize {
        PacketLayout::PACKET_LEN_SIZE + (u32::from_be_bytes(len) as usize)
    }

    pub fn decrypt_packet<'a>(&self, _pc: u64, buf: &'a mut [u8]) -> Option<&'a [u8]> {
        Some(&buf[PacketLayout::PACKET_LEN_SIZE ..])
    }
}
