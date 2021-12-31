use crate::util::codec::*;
use std::sync::Arc;
use zeroize::*;

#[derive(Clone)]
pub struct Secret(Arc<Vec<u8>>);

impl Secret {
    pub fn new(x: &[u8]) -> Self {
        Self(Arc::new(x.to_vec()))
    }
}

impl AsRef<[u8]> for Secret {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl SshEncode for Secret {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bytes_framed(self.0.as_ref())
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret(")?;
        for i in self.0.as_ref() {
            write!(f, "{:02x}", i)?;
        }
        write!(f, ")")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        if let Some(x) = Arc::get_mut(&mut self.0) {
            x.zeroize()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let x = Secret::new(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        assert_eq!(
            "Secret(000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f)",
            format!("{:?}", x)
        );
    }

    #[test]
    fn test_clone_01() {
        let x1 = Secret::new(&[
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
        let x = Secret::new(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        assert_eq!(&expected[..], &SshCodec::encode(&x).unwrap()[..]);
    }
}
