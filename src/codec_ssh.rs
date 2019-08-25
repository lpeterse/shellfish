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
        let mut v = Vec::with_capacity(len as usize);
        for _ in 0..len {
            v.push(SshCodec::decode(c)?);
        }
        Some(v)
    }
}
