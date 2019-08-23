pub struct Context<'a> (pub &'a [u8]);

impl <'a> Context<'a> {

    pub fn remaining(self: &Self) -> usize {
        self.0.len()
    }

    pub fn take<T>(self: &mut Self) -> Option<T>
        where
            T: Parser<'a>
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

    pub fn take_u32le(self: &mut Self) -> Option<u32> {
        let x = (*self.0.get(0)? as u32)
            |   (*self.0.get(1)? as u32) << 8
            |   (*self.0.get(2)? as u32) << 16
            |   (*self.0.get(3)? as u32) << 24;
        self.0 = &self.0[4..];
        Some(x)
    }

    pub fn take_vec<T>(self: &mut Self, n: usize) -> Option<Vec<T>>
        where
            T: Parser<'a>
    {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.take()?);
        }
        Some(v)
    }

    pub fn take_str(self: &mut Self, len: usize) -> Option<&'a str> {
        if self.0.len() < len {
            None
        } else {
            let (head, tail) = self.0.split_at(len);
            self.0 = tail;
            std::str::from_utf8(head).ok()
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

    pub fn take_len(self: &mut Self, len: usize) -> Option<&'a [u8]> {
        if self.0.len() < len {
            None
        } else {
            let r = &self.0[0..len];
            self.0 = &self.0[len..];
            Some(r)
        }
    }

    pub fn take_all(self: &mut Self) -> Option<&'a [u8]> {
        let r = &self.0[..];
        self.0 = &self.0[0..0];
        Some(r)
    }

    pub fn take_context(self: &mut Self, n: usize) -> Option<Context<'a>> {
        self.take_len(n).map(Context)
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

impl <'s> Parser<'s> for u8 {
    fn parse(c: &mut Context) -> Option<Self> {
        c.take_u8()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_remaining_01() {
        let a = [0,1,2,3 as u8];
        let c = Context(&a);

        assert_eq!(c.remaining(), 4);
    }

    #[test]
    fn test_context_take_u8_01() {
        let a = [0,1,2,3 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_u8(), Some(0));
        assert_eq!(c.take_u8(), Some(1));
        assert_eq!(c.take_u8(), Some(2));
        assert_eq!(c.take_u8(), Some(3));
        assert_eq!(c.take_u8(), None);
    }

    #[test]
    fn test_context_take_u32be_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_u32be(), Some(0x01020304));
        assert_eq!(c.take_u32be(), None);
    }

    #[test]
    fn test_context_take_u32le_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_u32le(), Some(0x04030201));
        assert_eq!(c.take_u32le(), None);
    }

    #[test]
    fn test_context_take_vec_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_vec(3), Some(vec![1,2,3 as u8]));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_vec_02() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_vec(5), Some(vec![1,2,3,4,5 as u8]));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_vec_03() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_vec(6), None as Option<Vec<u8>>);
    }

    #[test]
    fn test_context_take_str_01() {
        let a = "ABCDE".as_bytes();
        let mut c = Context(&a);

        assert_eq!(c.take_str(3), Some("ABC"));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_str_02() {
        let a = "ABCDE".as_bytes();
        let mut c = Context(&a);

        assert_eq!(c.take_str(5), Some("ABCDE"));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_str_03() {
        let a = "ABCDE".as_bytes();
        let mut c = Context(&a);

        assert_eq!(c.take_str(6), None);
    }

    #[test]
    fn test_context_take_string_01() {
        let a = "ABCDE".as_bytes();
        let mut c = Context(&a);

        assert_eq!(c.take_string(3), Some(String::from("ABC")));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_string_02() {
        let a = "ABCDE".as_bytes();
        let mut c = Context(&a);

        assert_eq!(c.take_string(5), Some(String::from("ABCDE")));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_string_03() {
        let a = "ABCDE".as_bytes();
        let mut c = Context(&a);

        assert_eq!(c.take_string(6), None);
    }

    #[test]
    fn test_context_take_len_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_len(3), Some(&[1,2,3 as u8][..]));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_len_02() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_len(5), Some(&[1,2,3,4,5 as u8][..]));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_len_03() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_len(6), None);
    }

    #[test]
    fn test_context_all_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Context(&a);

        assert_eq!(c.take_all(), Some(&[1,2,3,4,5 as u8][..]));
        assert_eq!(c.remaining(), 0);
    }
}