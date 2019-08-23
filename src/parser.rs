pub struct Context<'a> (pub &'a [u8]);

impl <'a> Context<'a> {

    pub fn remaining(self: &Self) -> usize {
        self.0.len()
    }

    pub fn take<A>(self: &mut Self) -> Option<A>
        where
            A: Parser<'a>
    {
        Parser::parse(self)
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

    pub fn take_n(self: &mut Self, n: usize) -> Option<&'a [u8]> {
        let r = &self.0[0..n];
        self.0 = &self.0[n..];
        Some(r)
    }

    pub fn take_all(self: &mut Self) -> Option<&'a [u8]> {
        let r = &self.0[..];
        self.0 = &self.0[0..0];
        Some(r)
    }

    pub fn take_context(self: &mut Self, n: usize) -> Option<Context<'a>> {
        self.take_n(n).map(Context)
    }
}

impl <'a> From<&'a [u8]> for Context<'a> {
    fn from(x: &'a [u8]) -> Self {
        Self(x)
    }
}

pub trait Parser<'s>: Sized {
    fn parse(c: &mut Context<'s>) -> Option<Self>;
}

impl <'s> Parser<'s> for usize {
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        let n = c.take_u32be()?;
        Some(n as usize)
    }
}

impl <'s> Parser<'s> for &'s [u8] {
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        c.take_all()
    }
}

impl <'s> Parser<'s> for Context<'s> 
{
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        let n: u32 = c.take_u32be()?;
        let r: &[u8] = c.take_n(n as usize)?;
        Some(Context(&r))
    }
}

impl <'s,T,Q> Parser<'s> for (T,Q)
    where 
        T: Parser<'s>,
        Q: Parser<'s>,
{
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        let t = Parser::parse(c)?;
        let q = Parser::parse(c)?;
        Some((t,q))
    }
}

impl <'s> Parser<'s> for String {
    fn parse(c: &mut Context) -> Option<Self> {
        let size = Parser::parse(c)?;
        c.take_string(size)
    }
}

impl <'s,T> Parser<'s> for Vec<T>
    where
        T: Parser<'s>,
{
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        let n = Parser::parse(c)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(Parser::parse(c)?);
        }
        Some(v)
    }
}

impl <'s> Parser<'s> for Vec<u8> {
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        let n = Parser::parse(c)?;
        c.take_vec(n)
    }
}
