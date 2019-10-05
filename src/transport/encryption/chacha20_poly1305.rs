use super::*;
use crate::transport::key_streams::*;

use chacha20::stream_cipher::{NewStreamCipher, SyncStreamCipher};
use chacha20::ChaCha20Legacy;
use poly1305::subtle::ConstantTimeEq;
use poly1305::Poly1305;
use std::convert::TryInto;

pub struct Chacha20Poly1305EncryptionContext {
    k1: [u8; 32],
    k2: [u8; 32],
}

impl Chacha20Poly1305EncryptionContext {
    const BLOCK_LEN: usize = 8;
    const MAC_LEN: usize = 16;

    pub fn new(ks: &mut KeyStream) -> Self {
        let mut k2: [u8; 32] = [0; 32];
        let mut k1: [u8; 32] = [0; 32];
        ks.read(&mut k2);
        ks.read(&mut k1);
        Self { k1, k2 }
    }

    pub fn new_keys(&mut self, ks: &mut KeyStream) {
        ks.read(&mut self.k2);
        ks.read(&mut self.k1);
    }

    pub fn buffer_layout(&self, payload_len: usize) -> PacketLayout {
        PacketLayout::new_aad(payload_len, Self::BLOCK_LEN, Self::MAC_LEN)
    }

    pub fn encrypt_packet(&self, pc: u64, layout: PacketLayout, buf: &mut [u8]) {
        // Insert packet len, padding len and padding
        layout.put_len(buf);
        layout.pad_zero(buf);
        // Encrypt packet length (first 4 bytes) with K1
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new_var(&self.k1, &nonce).unwrap();
        chacha.apply_keystream(&mut buf[layout.packet_len_range()]);
        // Compute Poly1305 key and create instance from the first 32 bytes of K2
        let mut chacha = ChaCha20Legacy::new_var(&self.k2, &nonce).unwrap();
        let mut poly_key: [u8; 32] = [0; 32];
        chacha.apply_keystream(&mut poly_key);
        let mut poly = Poly1305::new(&poly_key);
        // Consume the rest of the 1st chacha block
        chacha.apply_keystream(&mut poly_key);
        // Encipher padding len byte + msg + padding
        let packet = &mut buf[layout.cipher_range()];
        chacha.apply_keystream(packet);
        // Compute and set the Poly1305 auth tag
        poly.input(&buf[layout.integrity_range()]);
        let mac = &mut buf[layout.mac_range()];
        mac.copy_from_slice(poly.result().as_ref());
    }

    pub fn decrypt_len(&self, pc: u64, mut len: [u8; 4]) -> usize {
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new_var(&self.k1, &nonce).unwrap();
        chacha.apply_keystream(&mut len);
        PacketLayout::PACKET_LEN_SIZE + (u32::from_be_bytes(len) as usize) + Self::MAC_LEN
    }

    pub fn decrypt_packet<'a>(&self, pc: u64, buf: &'a mut [u8]) -> Option<&'a [u8]> {
        let buf_len = buf.len();
        let nonce: [u8; 8] = pc.to_be_bytes();
        // Compute Poly1305 key and create instance from the first 32 bytes of K2
        let mut chacha = ChaCha20Legacy::new_var(&self.k2, &nonce).unwrap();
        let mut poly_key: [u8; 32] = [0; 32];
        chacha.apply_keystream(&mut poly_key);
        let mut poly = Poly1305::new(&poly_key);
        chacha.apply_keystream(&mut poly_key); // consume the rest of the 1st chacha block
                                               // Compute and validate Poly1305 auth tag
        poly.input(&buf[..buf_len - Self::MAC_LEN]);
        let tag_computed = poly.result();
        let tag_received = &mut buf[buf_len - Self::MAC_LEN..];
        if tag_computed.as_ref().ct_eq(tag_received).unwrap_u8() == 1 {
            let packet = &mut buf[PacketLayout::PACKET_LEN_SIZE..buf_len - Self::MAC_LEN];
            chacha.apply_keystream(packet);
            Some(packet) // Message is authentic
        } else {
            None // Message is NOT authentic
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_eq() {
        let pc: u64 = 23;
        let payload: [u8; 17] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = Chacha20Poly1305EncryptionContext::new(&mut ks.c());
        // Encrypt
        let layout = ctx.buffer_layout(payload.len());
        let mut buf = Vec::new();
        buf.resize(layout.buffer_len(), 0);
        buf[layout.payload_range()].copy_from_slice(&payload);
        ctx.encrypt_packet(pc, layout, &mut buf);
        // Decrypt
        assert_eq!(
            Some(&payload[..]),
            ctx.decrypt_packet(pc, &mut buf)
                .map(|x| &x[1..][..payload.len()])
        );
    }

    #[test]
    fn test_encrypt_decrypt_bitflip() {
        let pc: u64 = 23;
        let payload: [u8; 17] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = Chacha20Poly1305EncryptionContext::new(&mut ks.c());
        // Encrypt
        let layout = ctx.buffer_layout(payload.len());
        let mut buf = Vec::new();
        buf.resize(layout.buffer_len(), 0);
        buf[layout.payload_range()].copy_from_slice(&payload);
        ctx.encrypt_packet(pc, layout, &mut buf);
        // Manipulate cipher text (single bit flip)
        buf[7] = buf[7] ^ 32;
        // Decrypt
        assert_eq!(None, ctx.decrypt_packet(pc, &mut buf));
    }

    #[test]
    fn test_poly1305() {
        let key = [
            2, 36, 186, 199, 156, 219, 160, 59, 58, 72, 185, 13, 36, 91, 46, 55, 10, 206, 108, 143,
            250, 250, 227, 41, 164, 26, 13, 4, 248, 136, 67, 35,
        ];
        let msg = [1, 2, 3, 4, 5, 6, 7, 8];
        let tag = [
            5, 144, 82, 159, 246, 206, 249, 18, 184, 150, 179, 37, 193, 39, 161, 138,
        ];
        let mut poly = Poly1305::new(&key);
        poly.input(&msg);
        assert_eq!(&tag, poly.result().as_ref());
    }
}
