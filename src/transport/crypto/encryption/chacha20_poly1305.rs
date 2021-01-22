use super::super::super::keys::*;
use super::*;
use crate::util::check;
use chacha20::cipher::stream::{NewStreamCipher, SyncStreamCipher};
use chacha20::ChaCha20Legacy;
use generic_array::GenericArray;
use poly1305::universal_hash::NewUniversalHash;
use poly1305::{Poly1305, Tag};
use zeroize::*;

type PolyKey = GenericArray<u8, <Poly1305 as NewUniversalHash>::KeySize>;

#[derive(Debug)]
pub struct Chacha20Poly1305Context {
    k1: [u8; 32],
    k2: [u8; 32],
}

impl Chacha20Poly1305Context {
    pub const BLOCK_LEN: usize = 8;
    pub const MAC_LEN: usize = 16;

    const PADDING_MIN_LEN: usize = 4;
    const PACKET_MIN_LEN: usize = 16;

    pub fn new(ks: &KeyStream) -> Self {
        let mut k2: [u8; 32] = [0; 32];
        let mut k1: [u8; 32] = [0; 32];
        ks.encryption_32_32(&mut k2, &mut k1);
        Self { k1, k2 }
    }

    pub fn update(&mut self, ks: &KeyStream) {
        ks.encryption_32_32(&mut self.k2, &mut self.k1);
    }

    pub fn encrypt(&self, pc: u64, buf: &mut [u8]) -> Result<(), TransportError> {
        const PACKET_LEN_BYTES: usize = 4;
        const ERR: TransportError = TransportError::InvalidEncryption;
        // Determine indices of length, authenticated, data and mac area
        let idx_len = ..PACKET_LEN_BYTES;
        let idx_auth = ..buf.len() - Self::MAC_LEN;
        let idx_data = PACKET_LEN_BYTES..buf.len() - Self::MAC_LEN;
        let idx_mac = buf.len() - Self::MAC_LEN..;
        // Encrypt packet length (first 4 bytes) with K1
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new((&self.k1).into(), (&nonce).into());
        chacha.apply_keystream(&mut buf.get_mut(idx_len).ok_or(ERR)?);
        // Compute Poly1305 key and create instance from the first 32 bytes of K2
        let mut chacha = ChaCha20Legacy::new((&self.k2).into(), (&nonce).into());
        let mut poly_key: PolyKey = [0; 32].into();
        chacha.apply_keystream(&mut poly_key);
        let poly = Poly1305::new(&poly_key);
        // Consume the rest of the 1st chacha block
        chacha.apply_keystream(&mut poly_key);
        // Encipher padding len byte + msg + padding
        chacha.apply_keystream(buf.get_mut(idx_data).ok_or(ERR)?);
        // Compute and set the Poly1305 auth tag
        let mac = poly
            .compute_unpadded(buf.get(idx_auth).ok_or(ERR)?)
            .into_bytes();
        buf.get_mut(idx_mac)
            .ok_or(ERR)?
            .copy_from_slice(mac.as_ref());
        Ok(())
    }

    pub fn decrypt(&self, pc: u64, buf: &mut [u8]) -> Result<(), TransportError> {
        const PACKET_LEN_BYTES: usize = 4;
        const ERR: TransportError = TransportError::InvalidEncryption;
        check(buf.len() > PACKET_LEN_BYTES + Self::MAC_LEN).ok_or(ERR)?;
        // Determine indices of authenticated, data and mac area
        let idx_auth = ..buf.len() - Self::MAC_LEN;
        let idx_data = PACKET_LEN_BYTES..buf.len() - Self::MAC_LEN;
        let idx_mac = buf.len() - Self::MAC_LEN..;
        // Compute Poly1305 key and create instance from the first 32 bytes of K2
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new((&self.k2).into(), (&nonce).into());
        let mut poly_key: PolyKey = [0; 32].into();
        chacha.apply_keystream(&mut poly_key);
        let poly = Poly1305::new(&poly_key);
        // Consume remainder of 1st chacha block
        chacha.apply_keystream(&mut poly_key);
        // Compute and validate Poly1305 auth tag
        let mac_observed = Tag::from(GenericArray::from_slice(buf.get(idx_mac).ok_or(ERR)?));
        let mac_computed = poly.compute_unpadded(buf.get(idx_auth).ok_or(ERR)?);
        // Check message integrity
        check(mac_computed == mac_observed).ok_or(ERR)?;
        // Decrypt and return data area len
        chacha.apply_keystream(buf.get_mut(idx_data).ok_or(ERR)?);
        Ok(())
    }

    pub fn decrypt_len(&self, pc: u64, mut len: [u8; 4]) -> Result<usize, TransportError> {
        let nonce: [u8; 8] = pc.to_be_bytes();
        let mut chacha = ChaCha20Legacy::new((&self.k1).into(), (&nonce).into());
        chacha.apply_keystream(&mut len);
        Ok(u32::from_be_bytes(len) as usize)
    }

    pub fn padding_len(&self, payload_len: usize) -> usize {
        let l = 1 + payload_len;
        let mut p = Self::BLOCK_LEN - (l % Self::BLOCK_LEN);
        if p < Self::PADDING_MIN_LEN {
            p += Self::BLOCK_LEN
        };
        while p + l < Self::PACKET_MIN_LEN {
            p += Self::BLOCK_LEN
        }
        p
    }
}

impl Zeroize for Chacha20Poly1305Context {
    fn zeroize(&mut self) {
        self.k1.zeroize();
        self.k2.zeroize();
    }
}

impl Drop for Chacha20Poly1305Context {
    fn drop(&mut self) {
        self.zeroize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let dir = KeyDirection::ClientToServer;
        let algo = KeyAlgorithm::Sha256;
        let k = [0u8; 32];
        let h = SessionId::new([0u8; 32]);
        let sid = SessionId::new([0u8; 32]);
        let ks = KeyStream::new(dir, algo, k, h, sid);
        let ctx = Chacha20Poly1305Context::new(&ks);

        assert_eq!(
            ctx.k1,
            [
                85, 81, 83, 240, 138, 99, 36, 217, 202, 127, 50, 172, 203, 46, 164, 128, 21, 133,
                223, 211, 200, 213, 89, 52, 64, 125, 127, 142, 70, 33, 40, 115
            ]
        );
        assert_eq!(
            ctx.k2,
            [
                139, 224, 214, 59, 139, 12, 205, 174, 32, 35, 203, 218, 65, 18, 110, 106, 130, 31,
                241, 34, 79, 188, 53, 185, 12, 230, 223, 30, 129, 126, 7, 229
            ]
        );
    }

    #[test]
    fn update() {
        // Initial context
        let dir = KeyDirection::ClientToServer;
        let algo = KeyAlgorithm::Sha256;
        let k = [0u8; 32];
        let h = SessionId::new([0u8; 32]);
        let sid = SessionId::new([0u8; 32]);
        let ks = KeyStream::new(dir, algo, k, h, sid);
        let mut ctx = Chacha20Poly1305Context::new(&ks);
        // Updated context
        let dir = KeyDirection::ClientToServer;
        let algo = KeyAlgorithm::Sha256;
        let k = [1u8; 32]; // <- sic!
        let h = SessionId::new([0u8; 32]);
        let sid = SessionId::new([0u8; 32]);
        let ks = KeyStream::new(dir, algo, k, h, sid);

        ctx.update(&ks);

        assert_eq!(
            ctx.k1,
            [
                119, 187, 217, 228, 158, 71, 184, 30, 179, 7, 206, 239, 67, 106, 37, 18, 60, 42,
                204, 177, 19, 172, 108, 227, 27, 65, 212, 146, 80, 79, 91, 81
            ]
        );
        assert_eq!(
            ctx.k2,
            [
                153, 191, 246, 179, 32, 250, 30, 173, 105, 136, 94, 221, 187, 96, 194, 129, 136,
                50, 33, 207, 25, 195, 181, 90, 197, 127, 62, 186, 234, 4, 58, 138
            ]
        );
    }

    #[test]
    fn encrypt_01() {
        let pc = 7;
        let ctx = Chacha20Poly1305Context {
            k1: [
                220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234,
                137, 133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
            ],
            k2: [
                136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7,
                131, 160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
            ],
        };
        let mut plain: [u8; 36] = [
            0, 0, 0, 16, 10, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let cipher: [u8; 36] = [
            76, 188, 158, 20, 126, 192, 194, 231, 77, 234, 102, 185, 54, 122, 208, 204, 155, 191,
            192, 209, 17, 47, 195, 149, 9, 143, 13, 207, 74, 6, 81, 152, 41, 219, 140, 154,
        ];

        ctx.encrypt(pc, &mut plain).unwrap();
        assert_eq!(&plain[..], &cipher[..]);
    }

    #[test]
    fn decrypt_len_01() {
        let pc = 7;
        let ctx = Chacha20Poly1305Context {
            k1: [
                220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234,
                137, 133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
            ],
            k2: [
                136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7,
                131, 160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
            ],
        };

        assert_eq!(ctx.decrypt_len(pc, [76, 188, 158, 20]).unwrap(), 16);
    }

    #[test]
    fn decrypt_valid() {
        let pc = 7;
        let ctx = Chacha20Poly1305Context {
            k1: [
                220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234,
                137, 133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
            ],
            k2: [
                136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7,
                131, 160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
            ],
        };
        let plain: [u8; 36] = [
            126, 246, 197, 155, 10, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 23, 27, 21, 224,
            187, 181, 146, 232, 50, 83, 6, 112, 219, 69, 113, 0,
        ];
        let mut cipher: [u8; 36] = [
            76, 188, 158, 20, 126, 192, 194, 231, 77, 234, 102, 185, 54, 122, 208, 204, 155, 191,
            192, 209, 17, 47, 195, 149, 9, 143, 13, 207, 74, 6, 81, 152, 41, 219, 140, 154,
        ];
        ctx.decrypt(pc, &mut cipher).unwrap();

        assert_eq!(&plain[4..20], &cipher[4..20]);
    }

    #[test]
    fn decrypt_invalid_mac() {
        let pc = 7;
        let ctx = Chacha20Poly1305Context {
            k1: [
                220, 134, 135, 208, 1, 2, 121, 163, 164, 252, 211, 244, 36, 148, 174, 220, 234,
                137, 133, 117, 40, 131, 157, 84, 211, 208, 74, 103, 215, 88, 145, 28,
            ],
            k2: [
                136, 155, 238, 35, 145, 72, 154, 220, 247, 70, 199, 97, 239, 124, 7, 41, 45, 7,
                131, 160, 203, 80, 54, 7, 100, 198, 188, 112, 19, 150, 155, 10,
            ],
        };
        let mut cipher: [u8; 36] = [
            76, 188, 158, 20, 126, 192, 194, 231, 77, 234, 102, 185, 54, 122, 208, 204, 155, 191,
            192, 209, 17, 47, 195, 149, 9, 143, 13, 207, 74, 6, 81, 152, 41, 219, 140,
            155, // <- !
        ];

        match ctx.decrypt(pc, &mut cipher) {
            Err(TransportError::InvalidEncryption) => (),
            Err(e) => panic!("unexpected error {:?}", e),
            _ => panic!("should have failed due to invalid mac"),
        }
    }
}
