use super::*;

pub struct Encoder<'a> {
    pub pos: usize,
    pub buf: &'a mut [u8],
}

impl <'a> Encoder<'a> {
    pub fn is_full(self: &Self) -> bool {
        self.pos >= self.buf.len()
    }

    pub fn remaining(self: &Self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn put_u8(self: &mut Self, x: u8) {
        self.buf[self.pos] = x;
        self.pos += 1;
    }

    pub fn put_u32be(self: &mut Self, x: u32) {
        self.buf[self.pos + 0] = (x >> 24) as u8;
        self.buf[self.pos + 1] = (x >> 16) as u8;
        self.buf[self.pos + 2] = (x >>  8) as u8;
        self.buf[self.pos + 3] = (x >>  0) as u8;
        self.pos += 4;
    }

    pub fn put_u32le(self: &mut Self, x: u32) {
        self.buf[self.pos + 0] = (x >>  0) as u8;
        self.buf[self.pos + 1] = (x >>  8) as u8;
        self.buf[self.pos + 2] = (x >> 16) as u8;
        self.buf[self.pos + 3] = (x >> 24) as u8;
        self.pos += 4;
    }

    pub fn put_bytes(self: &mut Self, x: &[u8]) {
        let b: &mut[u8] = &mut self.buf[self.pos..x.len()];
        b.copy_from_slice(x);
        self.pos += x.len();
    }

    pub fn put_str(self: &mut Self, x: &str) {
        self.put_bytes(x.as_bytes());
    }

    pub fn put_string(self: &mut Self, x: &String) {
        self.put_bytes(x.as_bytes());
    }

    pub fn put<T>(self: &mut Self, t: T)
        where
            T: Codec<'a>
    {
        t.encode(self)
    }
}

impl <'a> From<&'a mut [u8]> for Encoder<'a> {
    fn from(x: &'a mut [u8]) -> Self {
        Self {
            pos: 0,
            buf: x,
        }
    }
}
