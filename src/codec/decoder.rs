use super::*;

#[derive(Copy, Clone, Debug)]
pub struct Decoder<'a> (pub &'a [u8]);

impl <'a> Decoder<'a> {

    pub fn remaining(self: &Self) -> usize {
        self.0.len()
    }

    pub fn take<T>(self: &mut Self) -> Option<T>
        where
            T: Codec<'a>
    {
        Codec::decode(self)
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
            T: Codec<'a>
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

    pub fn take_bytes(self: &mut Self, len: usize) -> Option<&'a [u8]> {
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

    pub fn take_decoder(self: &mut Self, len: usize) -> Option<Self> {
        if self.0.len() < len {
            None
        } else {
            let r = &self.0[..len];
            self.0 = &self.0[len..];
            Some(Self(r))
        }
    }
}

impl <'a> From<&'a [u8]> for Decoder<'a> {
    fn from(x: &'a [u8]) -> Self {
        Self(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_remaining_01() {
        let a = [0,1,2,3 as u8];
        let c = Decoder(&a);

        assert_eq!(c.remaining(), 4);
    }

    #[test]
    fn test_context_take_u8_01() {
        let a = [0,1,2,3 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_u8(), Some(0));
        assert_eq!(c.take_u8(), Some(1));
        assert_eq!(c.take_u8(), Some(2));
        assert_eq!(c.take_u8(), Some(3));
        assert_eq!(c.take_u8(), None);
    }

    #[test]
    fn test_context_take_u32be_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_u32be(), Some(0x01020304));
        assert_eq!(c.take_u32be(), None);
    }

    #[test]
    fn test_context_take_u32le_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_u32le(), Some(0x04030201));
        assert_eq!(c.take_u32le(), None);
    }

/*
    #[test]
    fn test_context_take_vec_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_vec(3), Some(vec![1,2,3 as u8]));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_vec_02() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_vec(5), Some(vec![1,2,3,4,5 as u8]));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_vec_03() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_vec(6), None as Option<Vec<u8>>);
    }
*/

    #[test]
    fn test_context_take_str_01() {
        let a = "ABCDE".as_bytes();
        let mut c = Decoder(&a);

        assert_eq!(c.take_str(3), Some("ABC"));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_str_02() {
        let a = "ABCDE".as_bytes();
        let mut c = Decoder(&a);

        assert_eq!(c.take_str(5), Some("ABCDE"));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_str_03() {
        let a = "ABCDE".as_bytes();
        let mut c = Decoder(&a);

        assert_eq!(c.take_str(6), None);
    }

    #[test]
    fn test_context_take_string_01() {
        let a = "ABCDE".as_bytes();
        let mut c = Decoder(&a);

        assert_eq!(c.take_string(3), Some(String::from("ABC")));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_string_02() {
        let a = "ABCDE".as_bytes();
        let mut c = Decoder(&a);

        assert_eq!(c.take_string(5), Some(String::from("ABCDE")));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_string_03() {
        let a = "ABCDE".as_bytes();
        let mut c = Decoder(&a);

        assert_eq!(c.take_string(6), None);
    }

    #[test]
    fn test_context_take_bytes_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_bytes(3), Some(&[1,2,3 as u8][..]));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_bytes_02() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_bytes(5), Some(&[1,2,3,4,5 as u8][..]));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_bytes_03() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_bytes(6), None);
    }

    #[test]
    fn test_context_all_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = Decoder(&a);

        assert_eq!(c.take_all(), Some(&[1,2,3,4,5 as u8][..]));
        assert_eq!(c.remaining(), 0);
    }
}
