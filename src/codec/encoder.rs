use sha2::Digest;
use super::Encode;

pub trait Encoder {
    fn push_bool(&mut self, x: bool);
    fn push_u8(&mut self, x: u8);
    fn push_u32be(&mut self, x: u32);
    fn push_u32le(&mut self, x: u32);
    fn push_bytes<T: AsRef<[u8]>>(&mut self, x: &T);
}

impl <D: Digest> Encoder for D {
    fn push_bool(&mut self, x: bool) {
        self.input([if x { 1 } else { 0 }])
    }
    fn push_u8(&mut self, x: u8) {
        self.input([x])
    }
    fn push_u32be(&mut self, x: u32) {
        self.input(x.to_be_bytes())
    }
    fn push_u32le(&mut self, x: u32) {
        self.input(x.to_be_bytes())
    }
    fn push_bytes<T: AsRef<[u8]>>(&mut self, x: &T) {
        self.input(x)
    }
} 

#[derive(Debug)]
pub struct BEncoder<'a> {
    pub pos: usize,
    pub buf: &'a mut [u8],
}

impl <'a> BEncoder<'a> {
    pub fn encode<E: Encode>(e: &E) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.resize(e.size(), 0);
        let mut enc = BEncoder { pos: 0, buf: &mut vec };
        Encode::encode(e, &mut enc);
        vec
    }
}

impl <'a> Encoder for BEncoder<'a> {
    fn push_bool(&mut self, x: bool) {
        self.push_u8(if x { 1 } else { 0 })
    }

    fn push_u8(self: &mut Self, x: u8) {
        self.buf[self.pos] = x;
        self.pos += 1;
    }

    fn push_u32be(self: &mut Self, x: u32) {
        self.buf[self.pos + 0] = (x >> 24) as u8;
        self.buf[self.pos + 1] = (x >> 16) as u8;
        self.buf[self.pos + 2] = (x >>  8) as u8;
        self.buf[self.pos + 3] = (x >>  0) as u8;
        self.pos += 4;
    }

    fn push_u32le(self: &mut Self, x: u32) {
        self.buf[self.pos + 0] = (x >>  0) as u8;
        self.buf[self.pos + 1] = (x >>  8) as u8;
        self.buf[self.pos + 2] = (x >> 16) as u8;
        self.buf[self.pos + 3] = (x >> 24) as u8;
        self.pos += 4;
    }

    fn push_bytes<T: AsRef<[u8]>>(self: &mut Self, x: &T) {
        let b: &mut[u8] = &mut self.buf[self.pos..self.pos + x.as_ref().len()];
        b.copy_from_slice(x.as_ref());
        self.pos += x.as_ref().len();
    }
}

impl <'a> From<&'a mut [u8]> for BEncoder<'a> {
    fn from(x: &'a mut [u8]) -> Self {
        Self {
            pos: 0,
            buf: x,
        }
    }
}
