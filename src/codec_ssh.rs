use crate::codec::{Codec, Encoder, Decoder};

pub trait SshCode<'a>: Sized {
    fn size(&self) -> usize;
    fn encode(&self, c: &mut Encoder<'a>);
    fn decode(c: &mut Decoder<'a>) -> Option<Self>;
}

pub struct Ssh<T> (pub T);

impl <'a,T> Codec<'a> for Ssh<T>
    where
        T: SshCode<'a>
{
    fn size(&self) -> usize {
        self.0.size()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        self.0.encode(c)
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        SshCode::decode(c).map(Ssh)
    }
}

/*
impl <'s> Code<'s> for usize {
    fn size(&self) {}
    fn decode(c: &mut Decoder<'s>) -> Option<Self> {
        let n = c.take_u32be()?;
        Some(n as usize)
    }
}

impl <'s> Decode<'s> for String {
    fn decode(c: &mut Decoder) -> Option<Self> {
        let size = Decode::decode(c)?;
        c.take_string(size)
    }
}

impl <'s,T> Decode<'s> for Vec<T>
    where
        T: Decode<'s>,
{
    fn decode(c: &mut Decoder<'s>) -> Option<Self> {
        let n = Decode::decode(c)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(Decode::decode(c)?);
        }
        Some(v)
    }
}

impl <'s> Decode<'s> for Decoder<'s> 
{
    fn decode(c: &mut Decoder<'s>) -> Option<Self> {
        let n: u32 = c.take_u32be()?;
        let r: &[u8] = c.take_bytes(n as usize)?;
        Some(Decoder(&r))
    }
}
*/