use crate::parser::{Context, Parser};

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

impl <'s> Parser<'s> for Context<'s> 
{
    fn parse(c: &mut Context<'s>) -> Option<Self> {
        let n: u32 = c.take_u32be()?;
        let r: &[u8] = c.take_len(n as usize)?;
        Some(Context(&r))
    }
}