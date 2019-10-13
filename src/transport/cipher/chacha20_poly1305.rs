use super::*;
use crate::transport::key_streams::*;
use crate::util::assume;

use chacha20::stream_cipher::{NewStreamCipher, SyncStreamCipher};
use chacha20::ChaCha20Legacy;
use poly1305::subtle::ConstantTimeEq;
use poly1305::Poly1305;

pub struct Chacha20Poly1305Context {
    k1: [u8; 32],
    k2: [u8; 32],
}

impl Chacha20Poly1305Context {
    pub const AAD: bool = true;
    pub const BLOCK_LEN: usize = 8;
    pub const MAC_LEN: usize = 16;

    pub fn new(ks: &mut KeyStream) -> Self {
        let mut k2: [u8; 32] = [0; 32];
        let mut k1: [u8; 32] = [0; 32];
        ks.read(&mut k2);
        ks.read(&mut k1);
        Self { k1, k2 }
    }

    pub fn update(&mut self, ks: &mut KeyStream) {
        ks.read(&mut self.k2);
        ks.read(&mut self.k1);
    }

    pub fn encrypt(&self, pc: u64, buf: &mut [u8]) {
        // Encrypt packet length (first 4 bytes) with K1
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new_var(&self.k1, &nonce).unwrap();
        chacha.apply_keystream(&mut buf[..PacketLayout::PACKET_LEN_SIZE]);
        // Compute Poly1305 key and create instance from the first 32 bytes of K2
        let mut chacha = ChaCha20Legacy::new_var(&self.k2, &nonce).unwrap();
        let mut poly_key: [u8; 32] = [0; 32];
        chacha.apply_keystream(&mut poly_key);
        let mut poly = Poly1305::new(&poly_key);
        // Consume the rest of the 1st chacha block
        chacha.apply_keystream(&mut poly_key);
        // Encipher padding len byte + msg + padding
        let cipher_start = PacketLayout::PACKET_LEN_SIZE;
        let cipher_end = buf.len() - Self::MAC_LEN;
        chacha.apply_keystream(&mut buf[cipher_start..cipher_end]);
        // Compute and set the Poly1305 auth tag
        let integrity_start = 0;
        let integrity_end = cipher_end;
        poly.input(&buf[integrity_start..integrity_end]);
        let mac = &mut buf[integrity_end..];
        mac.copy_from_slice(poly.result().as_ref());
    }

    pub fn decrypt(&self, pc: u64, buf: &mut [u8]) -> Option<usize> {
        assume(buf.len() > PacketLayout::PACKET_LEN_SIZE + Self::MAC_LEN)?;
        let buf_len = buf.len();
        let nonce: [u8; 8] = pc.to_be_bytes();
        // Compute Poly1305 key and create instance from the first 32 bytes of K2
        let mut chacha = ChaCha20Legacy::new_var(&self.k2, &nonce).unwrap();
        let mut poly_key: [u8; 32] = [0; 32];
        chacha.apply_keystream(&mut poly_key);
        let mut poly = Poly1305::new(&poly_key);
        // Consume rest of 1st chacha block
        chacha.apply_keystream(&mut poly_key);
        // Compute and validate Poly1305 auth tag
        poly.input(&buf[..buf_len - Self::MAC_LEN]);
        let tag_computed = poly.result();
        let tag_received = &mut buf[buf_len - Self::MAC_LEN..];
        assume(tag_computed.as_ref().ct_eq(tag_received).unwrap_u8() == 1)?;
        let packet = &mut buf[PacketLayout::PACKET_LEN_SIZE..buf_len - Self::MAC_LEN];
        chacha.apply_keystream(packet);
        Some(buf.len() - PacketLayout::PACKET_LEN_SIZE - Self::MAC_LEN) // Message is authentic
    }

    pub fn decrypt_len(&self, pc: u64, mut len: [u8; 4]) -> Option<usize> {
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new_var(&self.k1, &nonce).unwrap();
        chacha.apply_keystream(&mut len);
        let len = u32::from_be_bytes(len) as usize;
        let len = PacketLayout::PACKET_LEN_SIZE + len + Self::MAC_LEN;
        assume(len <= PacketLayout::MAX_PACKET_LEN)?;
        Some(len)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new() {
        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = Chacha20Poly1305Context::new(&mut ks.c());
        assert_eq!(
            ctx.k1,
            [
                173, 191, 14, 248, 243, 145, 163, 223, 39, 37, 40, 156, 162, 23, 40, 136, 44, 116,
                35, 192, 159, 209, 196, 195, 238, 229, 27, 214, 96, 87, 212, 125
            ]
        );
        assert_eq!(
            ctx.k2,
            [
                228, 75, 14, 90, 219, 45, 123, 205, 221, 72, 66, 95, 217, 0, 83, 243, 254, 205,
                234, 128, 163, 38, 66, 235, 159, 133, 85, 193, 130, 109, 89, 100
            ]
        );
    }

    #[test]
    fn update() {
        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let mut ctx = Chacha20Poly1305Context::new(&mut ks.c());
        ctx.update(&mut ks.d());
        assert_eq!(
            ctx.k1,
            [
                64, 198, 150, 122, 78, 175, 56, 160, 162, 193, 208, 197, 21, 11, 23, 52, 240, 146,
                219, 132, 200, 175, 240, 167, 252, 98, 12, 219, 143, 97, 181, 228
            ]
        );
        assert_eq!(
            ctx.k2,
            [
                69, 185, 54, 154, 124, 158, 197, 187, 140, 130, 203, 250, 232, 158, 125, 83, 224,
                127, 234, 8, 184, 143, 137, 204, 181, 39, 244, 213, 253, 14, 38, 50
            ]
        );
    }

    #[test]
    fn encrypt_01() {
        let pc = 7;
        let k1 = [
            220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234, 137,
            133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
        ];
        let k2 = [
            136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7, 131,
            160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
        ];
        let mut plain: [u8; 36] = [
            0, 0, 0, 16, 10, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let cipher: [u8; 36] = [
            76, 188, 158, 20, 126, 192, 194, 231, 77, 234, 102, 185, 54, 122, 208, 204, 155, 191,
            192, 209, 17, 47, 195, 149, 9, 143, 13, 207, 74, 6, 81, 152, 41, 219, 140, 154,
        ];

        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let mut ctx = Chacha20Poly1305Context::new(&mut ks.c());
        ctx.k1 = k1;
        ctx.k2 = k2;
        ctx.encrypt(pc, &mut plain);
        assert_eq!(&plain[..], &cipher[..]);
    }

    #[test]
    fn decrypt_len_01() {
        let pc = 7;
        let k1 = [
            220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234, 137,
            133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
        ];
        let cipher: [u8; 4] = [76, 188, 158, 20];
        let plain: Option<usize> = Some(36);

        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let mut ctx = Chacha20Poly1305Context::new(&mut ks.c());
        ctx.k1 = k1;

        assert_eq!(plain, ctx.decrypt_len(pc, cipher));
    }

    #[test]
    fn decrypt_len_02() {
        let pc = 7;
        let cipher: [u8; 4] = [76, 188, 158, 20];
        let plain: Option<usize> = None;

        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = Chacha20Poly1305Context::new(&mut ks.c());

        assert_eq!(plain, ctx.decrypt_len(pc, cipher));
    }

    #[test]
    fn decrypt_01() {
        let pc = 7;
        let k1 = [
            220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234, 137,
            133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
        ];
        let k2 = [
            136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7, 131,
            160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
        ];
        let plain: [u8; 36] = [
            126, 246, 197, 155, 10, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 23, 27, 21, 224,
            187, 181, 146, 232, 50, 83, 6, 112, 219, 69, 113, 0,
        ];
        let mut cipher: [u8; 36] = [
            76, 188, 158, 20, 126, 192, 194, 231, 77, 234, 102, 185, 54, 122, 208, 204, 155, 191,
            192, 209, 17, 47, 195, 149, 9, 143, 13, 207, 74, 6, 81, 152, 41, 219, 140, 154,
        ];
        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let mut ctx = Chacha20Poly1305Context::new(&mut ks.c());
        ctx.k1 = k1;
        ctx.k2 = k2;
        let r = ctx.decrypt(pc, &mut cipher);
        assert_eq!(&plain[4..20], &cipher[4..20]);
        assert_eq!(Some(16), r);
    }

    #[test]
    fn decrypt_02() {
        let pc = 7;
        let k1 = [
            220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234, 137,
            133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
        ];
        let k2 = [
            136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7, 131,
            160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
        ];
        let mut cipher: [u8; 36] = [
            76, 188, 158, 20, 126, 192, 194, 231, 77, 234, 102, 185, 54, 122, 208, 204, 155, 191,
            192, 209, 17, 47, 195, 149, 9, 143, 13, 207, 74, 6, 81, 152, 41, 219, 140, 154,
        ];

        let mut ks = KeyStreams::new_sha256(&[], &[], &[]);
        let mut ctx = Chacha20Poly1305Context::new(&mut ks.c());
        ctx.k1 = k1;
        ctx.k2 = k2;
        cipher[8] += 1; // Introduce bitflip in ciphertext
        let r = ctx.decrypt(pc, &mut cipher);
        assert_eq!(None, r);
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
