pub struct Input<'a> (&'a [u8]);

impl <'a> Input<'a> {

    pub fn remaining(self: &Self) -> usize {
        self.0.len()
    }

    pub fn take_u8(self: &mut Self) -> Option<u8> {
        let (head, tail) = self.0.split_first()?;
        self.0 = tail;
        Some(*head)
    }

    pub fn take_u32be(self: &mut Self) -> Option<u32> {
        let x = (*self.0.get(0)? as u32) << 24
            |   (*self.0.get(1)? as u32) << 16
            |   (*self.0.get(2)? as u32) << 8
            |   (*self.0.get(3)? as u32);
        self.0 = &self.0[4..];
        Some(x)
    }

    pub fn take_vec(self: &mut Self, len: usize) -> Option<Vec<u8>> {
        if self.0.len() < len {
            None
        } else {
            let (head, tail) = self.0.split_at(len);
            self.0 = tail;
            Some(Vec::from(head))
        }
    }

    pub fn take_string(self: &mut Self, len: usize) -> Option<String> {
        if self.0.len() < len {
            None
        } else {
            let (head, tail) = self.0.split_at(len);
            self.0 = tail;
            String::from_utf8(Vec::from(head)).ok()
        }
    }
}

impl <'a> From<&'a mut [u8]> for Input<'a> {
    fn from(x: &'a mut [u8]) -> Self {
        Self(x)
    }
}

pub trait Parser: Sized {
    fn parse(p: &mut Input) -> Option<Self>;
}

impl Parser for usize {
    fn parse(p: &mut Input) -> Option<usize> {
        let i = p.take_u32be()?;
        Some(i as usize)
    }
}

impl Parser for String {
    fn parse(p: &mut Input) -> Option<String> {
        let size = Parser::parse(p)?;
        p.take_string(size)
    }
}

impl <T,Q> Parser for (T,Q) 
    where 
        T: Parser,
        Q: Parser,
{
    fn parse(p: &mut Input) -> Option<Self> {
        let t = Parser::parse(p)?;
        let q = Parser::parse(p)?;
        Some((t,q))
    }
}

impl <T> Parser for Vec<T>
    where
        T: Parser,
{
    fn parse(p: &mut Input) -> Option<Self> {
        let n = Parser::parse(p)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(Parser::parse(p)?);
        }
        Some(v)
    }
}

impl Parser for Vec<u8> {
    fn parse(p: &mut Input) -> Option<Self> {
        let n = Parser::parse(p)?;
        p.take_vec(n)
    }
}
