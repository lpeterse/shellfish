use crate::codec::*;

#[derive(Copy, Clone)]
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

impl std::fmt::Debug for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionId({:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x})",
            self.0[00], self.0[01], self.0[02], self.0[03],
            self.0[04], self.0[05], self.0[06], self.0[07],
            self.0[08], self.0[09], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15],
            self.0[16], self.0[17], self.0[18], self.0[19],
            self.0[20], self.0[21], self.0[22], self.0[23],
            self.0[24], self.0[25], self.0[26], self.0[27],
            self.0[28], self.0[29], self.0[30], self.0[31])
    }
}

impl Encode for SessionId {
    fn size(&self) -> usize {
        4 + 32
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(32);
        e.push_bytes(&self.as_ref());
    }
}

#[cfg(test)]
mod test {
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
        assert_eq!(&expected[..], &BEncoder::encode(&x)[..]);
    }
}
