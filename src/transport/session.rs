use crate::util::codec::*;

use zeroize::*;

#[derive(Clone)]
pub struct SessionId([u8; 32]);

impl SessionId {
    pub fn new(x: [u8; 32]) -> Self {
        Self(x)
    }
}

impl AsRef<[u8]> for SessionId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl SshEncode for SessionId {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bytes_framed(self.0.as_ref())
    }
}

impl std::fmt::Debug for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionId(")?;
        for i in &self.0 {
            write!(f, "{:02x}", i)?;
        }
        write!(f, ")")
    }
}

impl Zeroize for SessionId {
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl Drop for SessionId {
    fn drop(&mut self) {
        self.zeroize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let x = SessionId::new([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        assert_eq!(
            "SessionId(000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f)",
            format!("{:?}", x)
        );
    }

    #[test]
    fn test_clone_01() {
        let x1 = SessionId::new([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        let x2 = x1.clone();
        assert_eq!(x1.as_ref(), x2.as_ref());
    }

    #[test]
    fn test_encode_01() {
        let expected = [
            0, 0, 0, 32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        let x = SessionId::new([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        assert_eq!(&expected[..], &SshCodec::encode(&x).unwrap()[..]);
    }
}