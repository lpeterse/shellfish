pub trait Decoder<'a>: Clone {
    fn remaining(&self) -> usize;
    fn is_eoi(&self) -> bool;
    fn take_eoi(&self) -> Option<()>;
    fn take_u8(&mut self) -> Option<u8>;
    fn take_u32be(&mut self) -> Option<u32>;
    fn take_bool(&mut self) -> Option<bool> {
        self.take_u8().map(|x| x != 0)
    }
    fn take_bytes(&mut self, len: usize) -> Option<&'a [u8]>;
    fn take_all(&mut self) -> Option<&'a [u8]>;
    fn take_into(&mut self, buf: &mut [u8]) -> Option<()>;
    fn take_str(&mut self, len: usize) -> Option<&'a str>;
    fn take_string(&mut self, len: usize) -> Option<String>;
    fn take_while<F>(&mut self, pred: F) -> Option<&'a [u8]>
        where F: FnMut(u8) -> bool + Sized;
    fn take_match<T: AsRef<[u8]>>(&mut self, bytes: &T) -> Option<()>;
    // Convenience with default implementation
    fn expect_u8(&mut self, x: u8) -> Option<()> {
        self.take_u8().filter(|y| *y == x).map(drop)
    }
    fn expect_u32be(&mut self, x: u32) -> Option<()> {
        self.take_u32be().filter(|y| *y == x).map(drop)
    }
    fn expect_true(&mut self) -> Option<()> {
        self.take_u8().filter(|x| *x != 0).map(drop)
    }
    fn expect_false(&mut self) -> Option<()> {
        self.take_u8().filter(|x| *x == 0).map(drop)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BDecoder<'a> (pub &'a [u8]);

impl <'a> Decoder<'a> for BDecoder<'a> {
    fn remaining(&self) -> usize {
        self.0.len()
    }

    fn is_eoi(&self) -> bool {
        self.0.is_empty()
    }

    fn take_eoi(self: &Self) -> Option<()> {
        if self.is_eoi() { Some(()) } else { None }
    }

    fn take_match<T: AsRef<[u8]>>(self: &mut Self, bytes: &T) -> Option<()> {
        self.take_bytes(bytes.as_ref().len()).filter(|x| *x == bytes.as_ref()).map(drop)
    }

    fn take_u8(self: &mut Self) -> Option<u8> {
        let (head, tail) = self.0.split_first()?;
        self.0 = tail;
        Some(*head)
    }

    fn take_u32be(self: &mut Self) -> Option<u32> {
        let x = (*self.0.get(0)? as u32) << 24
            |   (*self.0.get(1)? as u32) << 16
            |   (*self.0.get(2)? as u32) << 8
            |   (*self.0.get(3)? as u32);
        self.0 = &self.0[4..];
        Some(x)
    }

    fn take_str(&mut self, len: usize) -> Option<&'a str> {
        if self.0.len() < len {
            None
        } else {
            let (head, tail) = self.0.split_at(len);
            self.0 = tail;
            std::str::from_utf8(head).ok()
        }
    }

    fn take_string(self: &mut Self, len: usize) -> Option<String> {
        if self.0.len() < len {
            None
        } else {
            let (head, tail) = self.0.split_at(len);
            self.0 = tail;
            String::from_utf8(Vec::from(head)).ok()
        }
    }

    fn take_bytes(self: &mut Self, len: usize) -> Option<&'a [u8]> {
        if self.0.len() < len {
            None
        } else {
            let r = &self.0[0..len];
            self.0 = &self.0[len..];
            Some(r)
        }
    }

    fn take_into(self: &mut Self, dst: &mut [u8]) -> Option<()> {
        let len = dst.len();
        if self.0.len() < len {
            None
        } else {
            dst.copy_from_slice(&self.0[..len]);
            self.0 = &self.0[len..];
            Some(())
        }
    }

    fn take_all(self: &mut Self) -> Option<&'a [u8]> {
        let r = &self.0[..];
        self.0 = &self.0[0..0];
        Some(r)
    }

    fn take_while<F>(self: &mut Self, mut pred: F) -> Option<&'a [u8]> 
        where
            F: FnMut(u8) -> bool + Sized
    {
        let mut i = 0;
        while i < self.0.len() && pred(self.0[i]) {
            i += 1;
        }
        let r = &self.0[..i];
        self.0 = &self.0[i..];
        Some(r)
    }
}

impl <'a> From<&'a [u8]> for BDecoder<'a> {
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
        let c = BDecoder(&a);

        assert_eq!(c.remaining(), 4);
    }

    #[test]
    fn test_context_take_u8_01() {
        let a = [0,1,2,3 as u8];
        let mut c = BDecoder(&a);

        assert_eq!(c.take_u8(), Some(0));
        assert_eq!(c.take_u8(), Some(1));
        assert_eq!(c.take_u8(), Some(2));
        assert_eq!(c.take_u8(), Some(3));
        assert_eq!(c.take_u8(), None);
    }

    #[test]
    fn test_context_take_u32be_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = BDecoder(&a);

        assert_eq!(c.take_u32be(), Some(0x01020304));
        assert_eq!(c.take_u32be(), None);
    }

    #[test]
    fn test_context_take_str_01() {
        let a = "ABCDE".as_bytes();
        let mut c = BDecoder(&a);

        assert_eq!(c.take_str(3), Some("ABC"));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_str_02() {
        let a = "ABCDE".as_bytes();
        let mut c = BDecoder(&a);

        assert_eq!(c.take_str(5), Some("ABCDE"));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_str_03() {
        let a = "ABCDE".as_bytes();
        let mut c = BDecoder(&a);

        assert_eq!(c.take_str(6), None);
    }

    #[test]
    fn test_context_take_string_01() {
        let a = "ABCDE".as_bytes();
        let mut c = BDecoder(&a);

        assert_eq!(c.take_string(3), Some(String::from("ABC")));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_string_02() {
        let a = "ABCDE".as_bytes();
        let mut c = BDecoder(&a);

        assert_eq!(c.take_string(5), Some(String::from("ABCDE")));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_string_03() {
        let a = "ABCDE".as_bytes();
        let mut c = BDecoder(&a);

        assert_eq!(c.take_string(6), None);
    }

    #[test]
    fn test_context_take_bytes_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = BDecoder(&a);

        assert_eq!(c.take_bytes(3), Some(&[1,2,3 as u8][..]));
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_context_take_bytes_02() {
        let a = [1,2,3,4,5 as u8];
        let mut c = BDecoder(&a);

        assert_eq!(c.take_bytes(5), Some(&[1,2,3,4,5 as u8][..]));
        assert_eq!(c.remaining(), 0);
    }

    #[test]
    fn test_context_take_bytes_03() {
        let a = [1,2,3,4,5 as u8];
        let mut c = BDecoder(&a);

        assert_eq!(c.take_bytes(6), None);
    }

    #[test]
    fn test_context_all_01() {
        let a = [1,2,3,4,5 as u8];
        let mut c = BDecoder(&a);

        assert_eq!(c.take_all(), Some(&[1,2,3,4,5 as u8][..]));
        assert_eq!(c.remaining(), 0);
    }
}
