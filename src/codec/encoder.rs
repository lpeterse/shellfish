use super::Encode;
use sha2::Digest;

pub trait Encoder: Sized {
    fn push_u8(&mut self, x: u8);
    fn push_u32be(&mut self, x: u32);
    fn push_u64be(&mut self, x: u64);
    fn push_bytes<T: AsRef<[u8]>>(&mut self, x: &T);
    fn push_encode<T: Encode>(&mut self, x: &T) {
        Encode::encode(x, self)
    }
}

impl<D: Digest> Encoder for D {
    fn push_u8(&mut self, x: u8) {
        self.input([x])
    }
    fn push_u32be(&mut self, x: u32) {
        self.input(x.to_be_bytes())
    }
    fn push_u64be(&mut self, x: u64) {
        self.input(x.to_be_bytes())
    }
    fn push_bytes<T: AsRef<[u8]>>(&mut self, x: &T) {
        self.input(x)
    }
}

#[derive(Debug)]
pub struct BEncoder<'a> {
    pos: usize,
    buf: &'a mut [u8],
}

impl<'a> BEncoder<'a> {
    pub fn encode<E: Encode>(e: &E) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.resize(e.size(), 0);
        let mut enc = BEncoder {
            pos: 0,
            buf: &mut vec,
        };
        Encode::encode(e, &mut enc);
        vec
    }
}

impl<'a> Encoder for BEncoder<'a> {
    fn push_u8(self: &mut Self, x: u8) {
        self.buf[self.pos] = x;
        self.pos += 1;
    }

    fn push_u32be(self: &mut Self, x: u32) {
        self.buf[self.pos + 0] = (x >> 24) as u8;
        self.buf[self.pos + 1] = (x >> 16) as u8;
        self.buf[self.pos + 2] = (x >> 8) as u8;
        self.buf[self.pos + 3] = (x >> 0) as u8;
        self.pos += 4;
    }

    fn push_u64be(self: &mut Self, x: u64) {
        self.buf[self.pos + 0] = (x >> 56) as u8;
        self.buf[self.pos + 1] = (x >> 48) as u8;
        self.buf[self.pos + 2] = (x >> 40) as u8;
        self.buf[self.pos + 3] = (x >> 32) as u8;
        self.buf[self.pos + 4] = (x >> 24) as u8;
        self.buf[self.pos + 5] = (x >> 16) as u8;
        self.buf[self.pos + 6] = (x >> 8) as u8;
        self.buf[self.pos + 7] = (x >> 0) as u8;
        self.pos += 8;
    }

    fn push_bytes<T: AsRef<[u8]>>(self: &mut Self, x: &T) {
        let input = x.as_ref();
        let b: &mut [u8] = &mut self.buf[self.pos..][..input.len()];
        b.copy_from_slice(input);
        self.pos += input.len();
    }
}

impl<'a> From<&'a mut [u8]> for BEncoder<'a> {
    fn from(x: &'a mut [u8]) -> Self {
        Self { pos: 0, buf: x }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bencoder_debug() {
        let enc = BEncoder::from(&mut [][..]);
        assert_eq!("BEncoder { pos: 0, buf: [] }", format!("{:?}", enc));
    }

    #[test]
    fn test_bencoder_push_u8() {
        let mut buf = [0; 2];
        let mut enc = BEncoder::from(&mut buf[..]);
        enc.push_u8(23);
        enc.push_u8(47);
        assert_eq!([23, 47], buf);
    }

    #[test]
    fn test_bencoder_push_u32be() {
        let mut buf = [0; 8];
        let mut enc = BEncoder::from(&mut buf[..]);
        enc.push_u32be(0x01020304);
        enc.push_u32be(0x05060708);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }

    #[test]
    fn test_bencoder_push_u64be() {
        let mut buf = [0; 16];
        let mut enc = BEncoder::from(&mut buf[..]);
        enc.push_u64be(0x0102030405060708);
        enc.push_u64be(0x0909090909090909);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9, 9, 9, 9, 9], buf);
    }

    #[test]
    fn test_bencoder_push_bytes() {
        let mut buf = [0; 8];
        let mut enc = BEncoder::from(&mut buf[..]);
        enc.push_bytes(&[1, 2, 3, 4]);
        enc.push_bytes(&[5, 6, 7, 8]);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }

    #[test]
    fn test_digest_debug() {
        let enc = BEncoder::from(&mut [][..]);
        assert_eq!("BEncoder { pos: 0, buf: [] }", format!("{:?}", enc));
    }

    #[test]
    fn test_digest_push_u8() {
        let mut digest = sha2::Sha256::new();
        digest.push_u8(0x01);
        assert_eq!([75, 245, 18, 47, 52, 69, 84, 197], digest.result()[..8]);
    }

    #[test]
    fn test_digest_push_u32be() {
        let mut digest = sha2::Sha256::new();
        digest.push_u32be(0x01020304);
        assert_eq!([159, 100, 167, 71, 225, 185, 127, 19], digest.result()[..8]);
    }
    #[test]
    fn test_digest_push_u64be() {
        let mut digest = sha2::Sha256::new();
        digest.push_u64be(0x0102030405060708);
        assert_eq!([102, 132, 13, 218, 21, 78, 138, 17], digest.result()[..8]);
    }

    #[test]
    fn test_digest_push_bytes() {
        let mut digest = sha2::Sha256::new();
        digest.push_bytes(&"1234");
        assert_eq!([3, 172, 103, 66, 22, 243, 225, 92], digest.result()[..8]);
    }
}
