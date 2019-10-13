use super::Encode;
use sha2::Digest;

pub trait Encoder {
    fn push_u8(&mut self, x: u8);
    fn push_u32be(&mut self, x: u32);
    fn push_bytes<T: AsRef<[u8]>>(&mut self, x: &T);
}

impl<D: Digest> Encoder for D {
    fn push_u8(&mut self, x: u8) {
        self.input([x])
    }
    fn push_u32be(&mut self, x: u32) {
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
mod test {
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
    fn test_bencoder_push_bytes() {
        let mut buf = [0; 8];
        let mut enc = BEncoder::from(&mut buf[..]);
        enc.push_bytes(&[1, 2, 3, 4]);
        enc.push_bytes(&[5, 6, 7, 8]);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], buf);
    }
}
