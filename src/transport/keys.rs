use crate::util::secret::Secret;
use crate::util::codec::*;
use sha2::{Digest, Sha256};
use zeroize::*;

/// The chapter "Output from Key Exchange" in the RFC describes how the parameters k, h and session
/// id are digested into several infinitely long key streams. This type offers methods for accessing
/// the different key types (initial, integrity and encryption). A [KeyStream] instance includes the
/// information about the direction "client to server" (or the other way around). This avoids
/// explicitly dispatching the client/server role in generic code (just pass the correct instance).
#[derive(Clone, Debug)]
pub struct KeyStream {
    dir: KeyDirection,
    algo: KeyAlgorithm,
    k: Secret,
    h: Secret,
    sid: Secret,
}

impl KeyStream {
    pub fn new_c2s(algo: KeyAlgorithm, k: &Secret, h: &Secret, sid: &Secret) -> Self {
        Self {
            dir: KeyDirection::ClientToServer,
            algo,
            k: k.clone(),
            h: h.clone(),
            sid: sid.clone(),
        }
    }

    pub fn new_s2c(algo: KeyAlgorithm, k: &Secret, h: &Secret, sid: &Secret) -> Self {
        Self {
            dir: KeyDirection::ServerToClient,
            algo,
            k: k.clone(),
            h: h.clone(),
            sid: sid.clone(),
        }
    }

    // Get the first 64 bytes from the encryption key stream (either 'C' or 'D').
    //
    // This methods is somewhat specialised as it writes the first and latter bytes to different
    // mutable destinations. Implement another method if you need more than 64 bytes!
    pub fn encryption_32_32(&self, k1: &mut [u8; 32], k2: &mut [u8; 32]) {
        let idx = match self.dir {
            KeyDirection::ClientToServer => 'C',
            KeyDirection::ServerToClient => 'D',
        };
        match self.algo {
            KeyAlgorithm::Sha256 => self.sha256_32_32(idx, k1, k2),
        }
    }

    fn sha256_32_32(&self, idx: char, k1: &mut [u8; 32], k2: &mut [u8; 32]) {
        let mut sha2 = Sha256::new();
        // RFC: "Here K is encoded as mpint and "A" as byte and session_id as raw
        //       data.  "A" means the single character A, ASCII 65."
        let _ = sha2.push_mpint(self.k.as_ref());
        sha2.update(self.h.as_ref());
        sha2.update([idx as u8]);
        sha2.update(self.sid.as_ref());
        let mut k1_ = sha2.finalize_reset();
        let _ = sha2.push_mpint(self.k.as_ref());
        sha2.update(self.h.as_ref());
        sha2.update(&k1_);
        let mut k2_ = sha2.finalize_reset();
        k1.copy_from_slice(&k1_[..]);
        k2.copy_from_slice(&k2_[..]);
        k1_.zeroize();
        k2_.zeroize();
    }
}

/// A finite list of supported hash algorithms.
///
/// Extend this list and the methods in [KeyStream] if necessary.
#[derive(Clone, Copy, Debug)]
pub enum KeyAlgorithm {
    Sha256,
}

/// The direction is either "client -> server" or "server -> client".
///
/// The direction information is included when creating keys and results in totally distinct keys.
#[derive(Clone, Copy, Debug)]
enum KeyDirection {
    ClientToServer,
    ServerToClient,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_streams_sha2_01() {
        let k = Secret::new(&[
            107, 228, 126, 33, 91, 152, 255, 218, 241, 220, 23, 167, 79, 146, 12, 100, 222, 142,
            141, 72, 246, 81, 24, 199, 127, 89, 24, 29, 124, 166, 187, 14,
        ]);
        let h = Secret::new(&[
            143, 162, 77, 88, 20, 122, 164, 90, 216, 15, 8, 149, 23, 47, 66, 157, 242, 12, 176, 63,
            153, 120, 103, 133, 17, 36, 10, 69, 6, 145, 250, 211,
        ]);

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

        let alg = KeyAlgorithm::Sha256;

        let key = KeyStream::new_c2s(alg, &k, &h, &h);
        let mut k1 = [0; 32];
        let mut k2 = [0; 32];
        key.encryption_32_32(&mut k1, &mut k2);
        assert_eq!(k1, c1, "c1");
        assert_eq!(k2, c2, "c2");

        let key = KeyStream::new_s2c(alg, &k, &h, &h);
        let mut k1 = [0; 32];
        let mut k2 = [0; 32];
        key.encryption_32_32(&mut k1, &mut k2);
        assert_eq!(k1, d1, "d1");
        assert_eq!(k2, d2, "d2");
    }

    #[test]
    fn test_key_streams_sha2_02() {
        let k = Secret::new(&[
            207, 228, 126, 33, 91, 152, 255, 218, 241, 220, 23, 167, 79, 146, 12, 100, 222, 142,
            //  ^ first byte of k is > 127
            141, 72, 246, 81, 24, 199, 127, 89, 24, 29, 124, 166, 187, 14,
        ]);
        let h = Secret::new(&[
            143, 162, 77, 88, 20, 122, 164, 90, 216, 15, 8, 149, 23, 47, 66, 157, 242, 12, 176, 63,
            153, 120, 103, 133, 17, 36, 10, 69, 6, 145, 250, 211,
        ]);

        let c1 = [
            125, 246, 53, 208, 237, 52, 170, 30, 97, 138, 151, 151, 199, 53, 83, 108, 130, 235,
            231, 87, 227, 10, 212, 137, 52, 16, 100, 244, 188, 104, 75, 76,
        ];

        let alg = KeyAlgorithm::Sha256;
        let key = KeyStream::new_c2s(alg, &k, &h.clone(), &h);
        let mut k1 = [0; 32];
        let mut k2 = [0; 32];
        key.encryption_32_32(&mut k1, &mut k2);
        assert_eq!(k1, c1, "c1");
    }
}
