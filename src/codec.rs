mod encoder;
mod decoder;

pub use self::encoder::*;
pub use self::decoder::*;

pub trait Codec<'a>: Sized {
    fn size(&self) -> usize;
    fn encode(&self, buf: &mut Encoder<'a>);
    fn decode(c: &mut Decoder<'a>) -> Option<Self>;
}

impl <'a,T,Q> Codec<'a> for (T,Q)
    where 
        T: Codec<'a>,
        Q: Codec<'a>,
{
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        self.0.encode(c);
        self.1.encode(c);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let t = Codec::decode(c)?;
        let q = Codec::decode(c)?;
        Some((t,q))
    }
}
