mod decoder;
mod encoder;

pub use self::decoder::*;
pub use self::encoder::*;

// FIXME
pub fn foobar(x: &mut [u8], y: u64) -> Option<()> {
    let mut a = SliceEncoder::new(x);
    a.push_u64be(y)?;
    a.push_u64be(y+1)?;
    a.push_bytes(&"abcfsdfsdfsdfsdfdef")?;
    Some(())
}

/// SSH specific encoding.
pub trait Encode {
    fn size(&self) -> usize;
    #[must_use]
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()>;
}

/// SSH specific decoding (inverse of `Encode`).
pub trait Decode: Sized {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self>;
}

/// SSH specific decode that allows the result to contain references into the input.
///
/// This is useful to avoid unnecessary intermediate allocations in cases where
/// the result is short-lived and can be processed while the input is still in scope.
pub trait DecodeRef<'a>: Sized {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self>;
}

/// `Decode  is a stronger property than `DecodeRef` so everything that is `Decode` can
/// automatically inherit `DecodeRef`.
impl<'a, T: Decode> DecodeRef<'a> for T {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Decode::decode(d)
    }
}

impl Encode for () {
    fn size(&self) -> usize {
        std::mem::size_of::<()>()
    }
    fn encode<E: Encoder>(&self, _: &mut E) -> Option<()> {
        Some(())
    }
}

impl Decode for () {
    fn decode<'a, D: Decoder<'a>>(_: &mut D) -> Option<Self> {
        Some(())
    }
}

impl Encode for u32 {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(*self)
    }
}

impl Decode for u32 {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u32be()
    }
}

impl Encode for u64 {
    fn size(&self) -> usize {
        std::mem::size_of::<u64>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u64be(*self)
    }
}

impl Decode for u64 {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u64be()
    }
}

impl Encode for String {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + self.len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) -> Option<()> {
        c.push_u32be(self.len() as u32)?;
        c.push_bytes(self)
    }
}

impl Decode for String {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_string(len as usize)
    }
}

impl Encode for &str {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + self.len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) -> Option<()> {
        c.push_u32be(self.len() as u32)?;
        c.push_bytes(self)
    }
}

impl<'a> DecodeRef<'a> for &'a str {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_str(len as usize)
    }
}

impl Encode for [u8] {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + self.len()
    }
    fn encode<E: Encoder>(&self, c: &mut E) -> Option<()> {
        c.push_u32be(self.len() as u32)?;
        c.push_bytes(&self)
    }
}

impl<'a> DecodeRef<'a> for &'a [u8] {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_bytes(len as usize)
    }
}

impl<T: Encode, Q: Encode> Encode for (T, Q) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
    fn encode<E: Encoder>(&self, c: &mut E) -> Option<()> {
        self.0.encode(c)?;
        self.1.encode(c)
    }
}

impl<T, Q> Decode for (T, Q)
where
    T: Decode,
    Q: Decode,
{
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let t = Decode::decode(c)?;
        let q = Decode::decode(c)?;
        Some((t, q))
    }
}

impl<A: Decode, B: Decode> Decode for Result<A, B> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        // The decoder state needs to be cloned as the input needs to be restored on failure.
        None.or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Ok);
            if r.is_some() {
                *d = d_
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Err);
            if r.is_some() {
                *d = d_
            };
            r
        })
    }
}

/// A vector is encoded by encoding its number of elements as u32 and each element according to its
/// own encoding rules.
impl<T: Encode> Encode for Vec<T> {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + self.iter().map(Encode::size).sum::<usize>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(self.len() as u32)?;
        for x in self {
            Encode::encode(x, e)?;
        }
        Some(())
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        // NB: Don't use `with_capacity` here as it might
        // lead to remote triggered resource exhaustion.
        let mut v = Vec::new();
        for _ in 0..len {
            v.push(DecodeRef::decode(c)?);
        }
        Some(v)
    }
}

/// `List` is a wrapper around `Vec` but with different encoding rules:
/// Instead of the number of elements, the leading u32 designates the following bytes.
pub struct List<T>(pub Vec<T>);

impl<T: Decode> Decode for List<T> {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        let bytes = c.take_bytes(len as usize)?;
        let mut vec: Vec<T> = Vec::new();
        let mut dec = SliceDecoder::new(&bytes);
        while let Some(s) = Decode::decode(&mut dec) {
            vec.push(s);
        }
        dec.take_eoi()?;
        Some(Self(vec))
    }
}

/// Like `List` but contains a reference to `Vec`.
pub struct ListRef<'a, T>(pub &'a Vec<T>);

impl<'a, T: Encode> Encode for ListRef<'a, T> {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + self.0.iter().map(Encode::size).sum::<usize>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        Encode::encode(&(self.0.iter().map(Encode::size).sum::<usize>() as u32), e)?;
        for x in self.0 {
            Encode::encode(x, e)?;
        }
        Some(())
    }
}

/// Certain lists in SSH (i.e. the SSH_KEX_INIT) message contain comma-separated ASCII lists.
/// This type contains operations for handling them.
pub enum NameList {}

impl NameList {
    pub fn size<T: AsRef<[u8]>>(vec: &Vec<T>) -> usize {
        let mut size = std::mem::size_of::<u32>();
        let mut names = vec.iter();
        if let Some(name) = names.next() {
            size += name.as_ref().len();
            for name in names {
                size += 1 + name.as_ref().len();
            }
        }
        size
    }
    #[must_use]
    pub fn encode<T: AsRef<[u8]>, E: Encoder>(vec: &Vec<T>, c: &mut E) -> Option<()> {
        c.push_u32be(NameList::size(vec) as u32 - std::mem::size_of::<u32>() as u32)?;
        let mut names = vec.iter();
        if let Some(name) = names.next() {
            c.push_bytes(name)?;
            for name in names {
                c.push_u8(',' as u8)?;
                c.push_bytes(name)?;
            }
        }
        Some(())
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
    pub fn decode_string<'a, D: Decoder<'a>>(c: &mut D) -> Option<Vec<String>> {
        let len = c.take_u32be()?;
        let mut vec = Vec::new();
        if len > 0 {
            let bytes: &'a [u8] = c.take_bytes(len as usize)?;
            for name in bytes.split(|c| c == &(',' as u8)) {
                vec.push(String::from_utf8(Vec::from(name)).ok()?);
            }
        }
        vec.into()
    }
}

/// FIXME: How does it relate to BigUint?
/// FIXME: Unit tests!
///
/// RFC 4251:
/// "Represents multiple precision integers in two's complement format,
/// stored as a string, 8 bits per byte, MSB first.  Negative numbers
/// have the value 1 as the most significant bit of the first byte of
/// the data partition.  If the most significant bit would be set for
/// a positive number, the number MUST be preceded by a zero byte.
/// Unnecessary leading bytes with the value 0 or 255 MUST NOT be
/// included.  The value zero MUST be stored as a string with zero
/// bytes of data."
pub struct MPInt<'a>(pub &'a [u8]);

impl<'a> Encode for MPInt<'a> {
    fn size(&self) -> usize {
        let mut x: &[u8] = self.0;
        while let Some(0) = x.get(0) {
            x = &x[1..];
        }
        if let Some(n) = x.get(0) {
            if *n > 127 {
                5 + x.len()
            } else {
                4 + x.len()
            }
        } else {
            4
        }
    }
    #[must_use]
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        let mut x: &[u8] = self.0;
        while let Some(0) = x.get(0) {
            x = &x[1..];
        }
        if let Some(n) = x.get(0) {
            if *n > 127 {
                e.push_u32be(x.len() as u32 + 1)?;
                e.push_u8(0)?;
            } else {
                e.push_u32be(x.len() as u32)?;
            }
            e.push_bytes(&x)?;
        } else {
            e.push_u32be(0)?;
        }
        Some(())
    }
}

impl<'a> DecodeRef<'a> for MPInt<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let len = d.take_u32be()? as usize;
        let bytes = d.take_bytes(len)?;
        if bytes[0] > 127 {
            // TODO: out of bounds
            Some(MPInt(&bytes[1..]))
        } else {
            Some(MPInt(bytes))
        }
    }
}
