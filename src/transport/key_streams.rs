use super::SessionId;
use crate::codec::*;

use sha2::{Digest, Sha256};
use std::sync::Arc;
use x25519_dalek::SharedSecret;
use zeroize::*;

#[derive(Clone, Debug)]
pub enum KeyStream {
    C2S(GenericKeyStream),
    S2C(GenericKeyStream),
}

impl KeyStream {
    pub fn encryption_32_32(&self, k1: &mut [u8; 32], k2: &mut [u8; 32]) {
        match self {
            Self::C2S(x) => x.key_32_32('C', k1, k2),
            Self::S2C(x) => x.key_32_32('D', k1, k2),
        }
    }
}

#[derive(Clone, Debug)]
pub enum GenericKeyStream {
    X25519(Arc<KeyStreamX25519>),
}

impl GenericKeyStream {
    pub fn c2s(&self) -> KeyStream {
        KeyStream::C2S(self.clone())
    }

    pub fn s2c(&self) -> KeyStream {
        KeyStream::S2C(self.clone())
    }

    pub fn key_32_32(&self, idx: char, k1: &mut [u8; 32], k2: &mut [u8; 32]) {
        match self {
            Self::X25519(x) => x.key_32_32(idx, k1, k2),
        }
    }
}

impl KeyStreamX25519 {
    pub fn new(k: SharedSecret, h: SessionId, sid: &SessionId) -> Self {
        let sid = sid.clone();
        Self { k, h, sid }
    }

    pub fn key_32_32(&self, idx: char, k1: &mut [u8; 32], k2: &mut [u8; 32]) {
        let mut sha2 = Sha256::new();
        // RFC: "Here K is encoded as mpint and "A" as byte and session_id as raw
        //       data.  "A" means the single character A, ASCII 65."
        Encode::encode(&MPInt(self.k.as_bytes()), &mut sha2);
        sha2.input(self.h.as_ref());
        sha2.input([idx as u8]);
        sha2.input(self.sid.as_ref());
        let mut k1_ = sha2.result_reset();
        Encode::encode(&MPInt(self.k.as_bytes()), &mut sha2);
        sha2.input(self.h.as_ref());
        sha2.input(&k1_);
        let mut k2_ = sha2.result_reset();
        k1.copy_from_slice(&k1_[..]);
        k2.copy_from_slice(&k2_[..]);
        k1_.zeroize();
        k2_.zeroize();
    }
}

pub struct KeyStreamX25519 {
    k: SharedSecret,
    h: SessionId,
    sid: SessionId,
}

impl std::fmt::Debug for KeyStreamX25519 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeyStreamX25519(...)")
    }
}

impl Into<GenericKeyStream> for KeyStreamX25519 {
    fn into(self) -> GenericKeyStream {
        GenericKeyStream::X25519(Arc::new(self))
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_streams_sha2_01() {
        let k = [
            107, 228, 126, 33, 91, 152, 255, 218, 241, 220, 23, 167, 79, 146, 12, 100, 222, 142,
            141, 72, 246, 81, 24, 199, 127, 89, 24, 29, 124, 166, 187, 14,
        ];
        let h = [
            143, 162, 77, 88, 20, 122, 164, 90, 216, 15, 8, 149, 23, 47, 66, 157, 242, 12, 176, 63,
            153, 120, 103, 133, 17, 36, 10, 69, 6, 145, 250, 211,
        ];
        let sid = SessionId::new(h);
        let c1 = [
            35, 83, 168, 202, 23, 231, 195, 6, 115, 123, 255, 191, 43, 255, 229, 67, 98, 137, 190,
            144, 108, 174, 108, 161, 250, 15, 170, 67, 142, 10, 102, 230,
        ];
        let c2 = [
            208, 1, 211, 152, 131, 216, 216, 233, 134, 111, 193, 40, 199, 147, 160, 146, 106, 253,
            28, 52, 89, 128, 0, 225, 61, 213, 79, 108, 116, 63, 4, 20,
        ];
        let d1 = [
            184, 79, 161, 100, 101, 46, 182, 213, 100, 94, 243, 107, 150, 176, 38, 24, 244, 253,
            153, 109, 83, 174, 5, 231, 139, 201, 30, 78, 88, 167, 227, 41,
        ];
        let d2 = [
            172, 158, 149, 34, 60, 215, 124, 232, 242, 63, 133, 44, 219, 188, 109, 5, 24, 230, 203,
            243, 189, 84, 85, 5, 162, 99, 163, 190, 201, 197, 78, 27,
        ];

        let ks = KeyStream::c2s_sha256(k.as_ref(), h, sid.clone());
        let mut c1_ = [0; 32];
        let mut c2_ = [0; 32];
        ks.encryption_32_32(&mut c1_, &mut c2_);
        assert_eq!(c1, c1_, "c1");
        assert_eq!(c2, c2_, "c2");

        let ks = KeyStream::s2c_sha256(k.as_ref(), h, sid);
        let mut d1_ = [0; 32];
        let mut d2_ = [0; 32];
        ks.encryption_32_32(&mut d1_, &mut d2_);
        assert_eq!(d1, d1_, "d1");
        assert_eq!(d2, d2_, "d2");
    }

    #[test]
    fn test_key_streams_sha2_02() {
        let k = [
            207, 228, 126, 33, 91, 152, 255, 218, 241, 220, 23, 167, 79, 146, 12, 100, 222, 142,
            141, 72, 246, 81, 24, 199, 127, 89, 24, 29, 124, 166, 187, 14,
        ];
        //  ^ first byte of k is > 127
        let h = [
            143, 162, 77, 88, 20, 122, 164, 90, 216, 15, 8, 149, 23, 47, 66, 157, 242, 12, 176, 63,
            153, 120, 103, 133, 17, 36, 10, 69, 6, 145, 250, 211,
        ];
        let sid = SessionId::new(h);
        let c1 = [
            125, 246, 53, 208, 237, 52, 170, 30, 97, 138, 151, 151, 199, 53, 83, 108, 130, 235,
            231, 87, 227, 10, 212, 137, 52, 16, 100, 244, 188, 104, 75, 76,
        ];

        let ks = KeyStream::c2s_sha256(k.as_ref(), h, sid);
        let mut c1_ = [0; 32];
        let mut c2_ = [0; 32];
        ks.encryption_32_32(&mut c1_, &mut c2_);
        assert_eq!(c1, c1_, "c1");
    }
}
*/
