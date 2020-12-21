mod decoder;
mod encoder;
mod ssh_decode;
mod ssh_encode;
mod ssh_encoder;

pub use self::decoder::*;
pub use self::encoder::*;
pub use self::ssh_decode::*;
pub use self::ssh_encode::*;
pub use self::ssh_encoder::*;

use crate::util::check;

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
        dec.expect_eoi()?;
        Some(Self(vec))
    }
}

/// Like `List` but contains a reference to `Vec`.
pub struct ListRef<'a, T>(pub &'a Vec<T>);

impl<'a, T: Encode> Encode for ListRef<'a, T> {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>() + self.0.iter().map(Encode::size).sum::<usize>()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_usize(self.0.iter().map(Encode::size).sum::<usize>())?;
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
            c.push_bytes(name.as_ref())?;
            for name in names {
                c.push_u8(',' as u8)?;
                c.push_bytes(name.as_ref())?;
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
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
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
