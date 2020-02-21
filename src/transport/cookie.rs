use rand::RngCore;
use rand::rngs::OsRng;

#[derive(Clone, Copy, PartialEq)]
pub struct KexCookie(pub [u8; 16]);

impl KexCookie {
    pub fn random() -> Self {
        let mut cookie: [u8; 16] = [0; 16];
        OsRng::default().fill_bytes(&mut cookie);
        Self(cookie)
    }
}

impl AsRef<[u8]> for KexCookie {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for KexCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KexCookie ({:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x})",
            self.0[00], self.0[01], self.0[02], self.0[03],
            self.0[04], self.0[05], self.0[06], self.0[07],
            self.0[08], self.0[09], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_random_01() {
        let c1 = KexCookie::random();
        let c2 = KexCookie::random();
        assert_ne!(c1.0, c2.0);
    }

    #[test]
    fn test_clone_01() {
        let k1 = KexCookie([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let k2 = k1.clone();
        assert_eq!(k1.0, k2.0);
    }

    #[test]
    fn test_debug_01() {
        let k1 = KexCookie([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        assert_eq!(
            "KexCookie (000102030405060708090a0b0c0d0e0f)",
            format!("{:?}", k1)
        );
    }
}
