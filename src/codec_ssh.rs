use num::BigUint;

use crate::codec::{Encoder, Decoder};

pub trait SshCodec<'a>: Sized {
    fn size(&self) -> usize;
    fn encode(&self, c: &mut Encoder<'a>);
    fn decode(c: &mut Decoder<'a>) -> Option<Self>;
}

impl <'a> SshCodec<'a> for String {
    fn size(&self) -> usize {
        4 + self.as_bytes().len()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u32be(self.as_bytes().len() as u32);
        c.push_string(self);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_string(len as usize)
    }
}

impl <'a> SshCodec<'a> for &'a str {
    fn size(&self) -> usize {
        4 + self.as_bytes().len()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u32be(self.as_bytes().len() as u32);
        c.push_str(self);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_str(len as usize)
    }
}

impl <'a,T,Q> SshCodec<'a> for (T,Q)
    where 
        T: SshCodec<'a>,
        Q: SshCodec<'a>,
{
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        self.0.encode(c);
        self.1.encode(c);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let t = SshCodec::decode(c)?;
        let q = SshCodec::decode(c)?;
        Some((t,q))
    }
}

impl <'a> SshCodec<'a> for Vec<u8>
{
    fn size(&self) -> usize {
        4 + self.len()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u32be(self.len() as u32);
        c.push_bytes(self.as_slice());
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        Some(Vec::from(c.take_bytes(len as usize)?))
    }
}

impl <'a,T> SshCodec<'a> for Vec<T>
    where
        T: SshCodec<'a>,
{
    fn size(&self) -> usize {
        let mut r = 4;
        for x in self {
            r += x.size();
        }
        r
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u32be(self.len() as u32);
        for x in self {
            SshCodec::encode(x, c);
        }
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        // NB: Don't use `with_capacity` here as it might
        // lead to remote triggered resource exhaustion
        let mut v = Vec::new();
        for _ in 0..len {
            v.push(SshCodec::decode(c)?);
        }
        Some(v)
    }
}

impl <'a> SshCodec<'a> for BigUint
{
    fn size(&self) -> usize {
        let vec = self.to_bytes_be();
        let bytes = vec.as_slice();
        if bytes[0] > 127 {
            4 + 1 + bytes.len()
        } else {
            4 + bytes.len()
        }
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        let vec = self.to_bytes_be();
        let bytes = vec.as_slice(); // bytes is non-empty
        if bytes[0] > 127 {
            c.push_u32be(1 + bytes.len() as u32);
            c.push_u8(0);
            c.push_bytes(bytes);
        } else {
            c.push_u32be(bytes.len() as u32);
            c.push_bytes(bytes);
        }
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        let bytes = c.take_bytes(len as usize)?;
        if bytes.is_empty() {
            Some(Self::from(0 as usize))
        } else {
            let mut i = 0;
            while i < bytes.len() && bytes[i] == 0 { i += 1 };
            Some(BigUint::from_bytes_be(&bytes[i..]))
        }
    }
}

pub enum NameList {}

impl NameList {
    pub fn size<T: AsRef<[u8]>>(vec: &Vec<T>) -> usize {
        let mut size = 4;
        let mut names = vec.iter();
        if let Some(name) = names.next() {
            size += name.as_ref().len();
            for name in names {
                size += 1 + name.as_ref().len();
            }
        }
        size
    }
    pub fn encode<T: AsRef<[u8]>>(vec: &Vec<T>, c: &mut Encoder) {
        c.push_u32be(NameList::size(vec) as u32 - 4);
        let mut names = vec.iter();
        if let Some(name) = names.next() {
            c.push_bytes(name.as_ref());
            for name in names {
                c.push_u8(',' as u8);
                c.push_bytes(name.as_ref());
            }
        }
    }
    pub fn decode<'a, T: std::convert::TryFrom<&'a [u8]>>(c: &mut Decoder<'a>) -> Option<Vec<T>> {
        let len = c.take_u32be()?;
        let bytes = c.take_bytes(len as usize)?;
        let mut vec = Vec::new();
        for name in bytes.split(|c| c == &(',' as u8)) {
            vec.push(std::convert::TryFrom::try_from(name).ok()?);
        }
        vec.into()
    }
}
