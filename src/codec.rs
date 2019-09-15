mod encoder;
mod decoder;

use num::BigUint;

pub use self::encoder::*;
pub use self::decoder::*;

pub trait Encode {
    fn size(&self) -> usize;
    fn encode<E: Encoder>(&self, c: &mut E);
}

pub trait Decode<'a>: Encode + Sized {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self>;
}

impl Encode for () {
    fn size(&self) -> usize {
        0
    }
    fn encode<E: Encoder>(&self, _: &mut E) {
        // Nothing to do
    }
}

impl Encode for u8 {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(*self)
    }
}

impl Encode for u32 {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(*self)
    }
}

impl <'a> Decode<'a> for () {
    fn decode<D: Decoder<'a>>(_: &mut D) -> Option<Self> {
        Some(())
    }
}

impl Encode for String {
    fn size(&self) -> usize {
        4 + self.as_bytes().len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(self.as_bytes().len() as u32);
        c.push_bytes(self);
    }
}

impl <'a> Decode<'a> for String {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_string(len as usize)
    }
}

impl Encode for &str {
    fn size(&self) -> usize {
        4 + self.as_bytes().len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(self.as_bytes().len() as u32);
        c.push_bytes(self);
    }
}

impl <'a> Decode<'a> for &'a str {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_str(len as usize)
    }
}

impl Encode for &[u8] {
    fn size(&self) -> usize {
        4 + self.len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(self.len() as u32);
        c.push_bytes(self);
    }
}

impl <'a> Decode<'a> for &'a [u8] {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_bytes(len as usize)
    }
}

impl <T: Encode, Q:Encode> Encode for (T,Q) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        self.0.encode(c);
        self.1.encode(c);
    }
}

impl <'a,T,Q> Decode<'a> for (T,Q)
    where 
        T: Decode<'a>,
        Q: Decode<'a>,
{
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let t = Decode::decode(c)?;
        let q = Decode::decode(c)?;
        Some((t,q))
    }
}

impl <T: Encode> Encode for Vec<T> {
    fn size(&self) -> usize {
        let mut r = 4;
        for x in self {
            r += x.size();
        }
        r
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(self.len() as u32);
        for x in self {
            Encode::encode(x, c);
        }
    }
}

impl <'a,T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        // NB: Don't use `with_capacity` here as it might
        // lead to remote triggered resource exhaustion
        let mut v = Vec::new();
        for _ in 0..len {
            v.push(Decode::decode(c)?);
        }
        Some(v)
    }
}

impl Encode for BigUint {
    fn size(&self) -> usize {
        let vec = self.to_bytes_be();
        let bytes = vec.as_slice();
        if bytes[0] > 127 {
            4 + 1 + bytes.len()
        } else {
            4 + bytes.len()
        }
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        let vec = self.to_bytes_be();
        let bytes = vec.as_slice(); // bytes is non-empty
        if bytes[0] > 127 {
            c.push_u32be(1 + bytes.len() as u32);
            c.push_u8(0);
            c.push_bytes(&bytes);
        } else {
            c.push_u32be(bytes.len() as u32);
            c.push_bytes(&bytes);
        }
    }
}

impl <'a> Decode<'a> for BigUint {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
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
    pub fn encode<T: AsRef<[u8]>, E: Encoder>(vec: &Vec<T>, c: &mut E) {
        c.push_u32be(NameList::size(vec) as u32 - 4);
        let mut names = vec.iter();
        if let Some(name) = names.next() {
            c.push_bytes(name);
            for name in names {
                c.push_u8(',' as u8);
                c.push_bytes(name);
            }
        }
    }
    pub fn decode<'a, T: std::convert::TryFrom<&'a [u8]>, D: Decoder<'a>>(c: &mut D) -> Option<Vec<T>> {
        let len = c.take_u32be()?;
        let mut vec = Vec::new();
        if len > 0 {
            let bytes = c.take_bytes(len as usize)?;
            for name in bytes.split(|c| c == &(',' as u8)) {
                vec.push(std::convert::TryFrom::try_from(name).ok()?);
            }
        }
        vec.into()
    }
    pub fn decode_str<'a, D: Decoder<'a>>(c: &mut D) -> Option<Vec<&'a str>> {
        let len = c.take_u32be()?;
        let mut vec = Vec::new();
        if len > 0 {
            let bytes: &'a [u8] = c.take_bytes(len as usize)?;
            for name in bytes.split(|c| c == &(',' as u8)) {
                vec.push(std::str::from_utf8(name).ok()?);
            }
        }
        vec.into()
    }
}

pub struct MPInt<'a> (pub &'a [u8]);

impl <'a> Encode for MPInt<'a> {
    fn size(&self) -> usize {
        4 + if self.0[0] > 127 { 1 } else { 0 } + self.0.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        let len = self.0.len();
        if self.0[0] > 127 {
            e.push_u32be(len as u32 + 1);
            e.push_u8(0);
        } else {
            e.push_u32be(len as u32);
        }
        e.push_bytes(&self.0);
    }
}

impl <'a> Decode<'a> for MPInt<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let len = d.take_u32be()? as usize;
        let bytes = d.take_bytes(len)?;
        if bytes[0] > 127 { // TODO: out of bounds
            Some(MPInt(&bytes[1..]))
        } else {
            Some(MPInt(bytes))
        }
    }
}

pub enum E2<A,B> {
    A(A),
    B(B),
}

impl <A: Encode,B: Encode> Encode for E2<A,B> {
    fn size(&self) -> usize {
        match self {
            Self::A(x) => Encode::size(x),
            Self::B(x) => Encode::size(x),
        }
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        match self {
            Self::A(x) => Encode::encode(x,c),
            Self::B(x) => Encode::encode(x,c)
        }
    }
}

impl <'a,A: Decode<'a>,B: Decode<'a>> Decode<'a> for E2<A,B> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        None.or_else(|| {
                let mut d_ = d.clone();
                let r = Decode::decode(&mut d_).map(Self::A);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = Decode::decode(&mut d_).map(Self::B);
                if r.is_some() { *d = d_ };
                r
            })
    }
}

pub enum E3<A,B,C> {
    A(A),
    B(B),
    C(C),
}

impl <A: Encode, B:Encode, C:Encode> Encode for E3<A,B,C> {
    fn size(&self) -> usize {
        match self {
            Self::A(x) => Encode::size(x),
            Self::B(x) => Encode::size(x),
            Self::C(x) => Encode::size(x),
        }
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        match self {
            Self::A(x) => Encode::encode(x,c),
            Self::B(x) => Encode::encode(x,c),
            Self::C(x) => Encode::encode(x,c)
        }
    }
}

impl <'a,A,B,C> Decode<'a> for E3<A,B,C>
    where
        A: Decode<'a>,
        B: Decode<'a>,
        C: Decode<'a>,
{

    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        None.or_else(|| {
                let mut d_ = d.clone();
                let r = Decode::decode(&mut d_).map(Self::A);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = Decode::decode(&mut d_).map(Self::B);
                if r.is_some() { *d = d_ };
                r
            })
            .or_else(|| {
                let mut d_ = d.clone();
                let r = Decode::decode(&mut d_).map(Self::C);
                if r.is_some() { *d = d_ };
                r
            })
    }
}
