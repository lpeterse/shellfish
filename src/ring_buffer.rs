pub struct RingBuffer {
    max_capacity: usize,
    off: usize,
    len: usize,
    buf: Box<[u8]>,
}

impl RingBuffer {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            max_capacity,
            off: 0,
            len: 0,
            // Initialize empty in expectation the buffer will never be used.
            buf: Vec::new().into_boxed_slice(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.max_capacity
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        if buf.is_empty() || self.is_empty() {
            0
        } else if self.off + self.len <= self.buf.len() {
            let l = std::cmp::min(buf.len(), self.len);
            buf[..l].copy_from_slice(&self.buf[self.off..][..l]);
            self.len -= l;
            self.off = if self.len == 0 {
                0
            } else {
                (self.off + l) % self.buf.len()
            };
            l
        } else {
            let l1 = std::cmp::min(buf.len(), self.buf.len() - self.off);
            let l2 = std::cmp::min(buf.len() - l1, self.len - l1);
            let l3 = l1 + l2;
            buf[..l1].copy_from_slice(&self.buf[self.off..][..l1]);
            buf[l1..][..l2].copy_from_slice(&self.buf[..l2]);
            self.len -= l3;
            self.off = if self.len == 0 {
                0
            } else {
                (self.off + l3) % self.buf.len()
            };
            l3
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> usize {
        if buf.is_empty() || self.is_full() {
            return 0;
        }
        let requested = buf.len();
        let available = self.buf.len() - self.len();
        if available < requested {
            self.grow((buf.len() - available) * 2);
        }
        let available = self.buf.len() - self.len();
        let written = std::cmp::min(available, requested);
        let l1 = std::cmp::min(written, self.buf.len() - (self.off + self.len));
        let l2 = written - l1;
        self.buf[self.off + self.len..][..l1].copy_from_slice(&buf[..l1]);
        self.buf[..l2].copy_from_slice(&buf[l1..][..l2]);
        self.len += written;
        written
    }

    fn grow(&mut self, increase: usize) {
        assert!(!self.is_full());
        assert!(self.buf.len() != self.max_capacity);

        let old_len = self.buf.len();
        let new_len = std::cmp::min(old_len + increase, self.max_capacity);
        let mut new_buf = {
            let mut v = Vec::with_capacity(new_len);
            v.resize(new_len, 0);
            v.into_boxed_slice()
        };
        if self.off + self.len <= old_len {
            new_buf[..self.len].copy_from_slice(&self.buf[self.off..][..self.len]);
        } else {
            let l1 = old_len - self.off;
            let l2 = self.len - l1;
            new_buf[..l1].copy_from_slice(&self.buf[self.off..][..l1]);
            new_buf[l1..][..l2].copy_from_slice(&self.buf[..l2]);
        }
        self.buf = new_buf;
        self.off = 0;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_empty_01() {
        let x = RingBuffer::new(32);
        assert!(x.is_empty())
    }

    #[test]
    fn test_is_empty_02() {
        let mut x = RingBuffer::new(32);
        x.len = 1;
        assert!(!x.is_empty())
    }

    #[test]
    fn test_is_full_01() {
        let x = RingBuffer::new(32);
        assert!(!x.is_full());
    }

    #[test]
    fn test_is_full_02() {
        let mut x = RingBuffer::new(32);
        x.len = 31;
        assert!(!x.is_full());
    }

    #[test]
    fn test_is_full_03() {
        let mut x = RingBuffer::new(32);
        x.len = 32;
        assert!(x.is_full());
    }

    #[test]
    fn test_is_len_01() {
        let x = RingBuffer::new(32);
        assert_eq!(0, x.len());
    }

    #[test]
    fn test_is_len_02() {
        let mut x = RingBuffer::new(32);
        x.len = 23;
        assert_eq!(23, x.len());
    }

    // Scenario: Passed buffer empty
    #[test]
    fn test_read_01() {
        let mut x = RingBuffer::new(32);
        x.len = 1;
        let mut b = [];
        assert_eq!(0, x.read(&mut b));
    }

    // Scenario: Internal buffer empty
    #[test]
    fn test_read_02() {
        let mut x = RingBuffer::new(32);
        let mut b = [0];
        assert_eq!(0, x.read(&mut b));
    }

    // Scenario: No overlap, request less
    #[test]
    fn test_read_03() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0].into_boxed_slice();
        x.off = 3;
        x.len = 7;
        let mut b = [9, 9, 9];
        assert_eq!(3, x.read(&mut b));
        assert_eq!([1, 2, 3], b);
        assert_eq!(6, x.off);
        assert_eq!(4, x.len);
    }

    // Scenario: No overlap, request equal
    #[test]
    fn test_read_04() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0].into_boxed_slice();
        x.off = 3;
        x.len = 7;
        let mut b = [9, 9, 9, 9, 9, 9, 9];
        assert_eq!(7, x.read(&mut b));
        assert_eq!([1, 2, 3, 4, 5, 6, 7], b);
        assert_eq!(0, x.off);
        assert_eq!(0, x.len);
    }

    // Scenario: No overlap, request more
    #[test]
    fn test_read_05() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0].into_boxed_slice();
        x.off = 3;
        x.len = 7;
        let mut b = [9, 9, 9, 9, 9, 9, 9, 9];
        assert_eq!(7, x.read(&mut b));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 9], b);
        assert_eq!(0, x.off);
        assert_eq!(0, x.len);
    }

    // Scenario: Overlap, request less than first slice
    #[test]
    fn test_read_06() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![5, 6, 7, 0, 0, 0, 0, 1, 2, 3, 4].into_boxed_slice();
        x.off = 7;
        x.len = 7;
        let mut b = [9, 9, 9];
        assert_eq!(3, x.read(&mut b));
        assert_eq!([1, 2, 3], b);
        assert_eq!(10, x.off);
        assert_eq!(4, x.len);
    }

    // Scenario: Overlap, request whole first slice
    #[test]
    fn test_read_07() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![5, 6, 7, 0, 0, 0, 0, 1, 2, 3, 4].into_boxed_slice();
        x.off = 7;
        x.len = 7;
        let mut b = [9, 9, 9, 9];
        assert_eq!(4, x.read(&mut b));
        assert_eq!([1, 2, 3, 4], b);
        assert_eq!(0, x.off);
        assert_eq!(3, x.len);
    }

    // Scenario: Overlap, request less
    #[test]
    fn test_read_08() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![5, 6, 7, 0, 0, 0, 0, 1, 2, 3, 4].into_boxed_slice();
        x.off = 7;
        x.len = 7;
        let mut b = [9, 9, 9, 9, 9];
        assert_eq!(5, x.read(&mut b));
        assert_eq!([1, 2, 3, 4, 5], b);
        assert_eq!(1, x.off);
        assert_eq!(2, x.len);
    }

    // Scenario: Overlap, request equal
    #[test]
    fn test_read_09() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![5, 6, 7, 0, 0, 0, 0, 1, 2, 3, 4].into_boxed_slice();
        x.off = 7;
        x.len = 7;
        let mut b = [9, 9, 9, 9, 9, 9, 9];
        assert_eq!(7, x.read(&mut b));
        assert_eq!([1, 2, 3, 4, 5, 6, 7], b);
        assert_eq!(0, x.off);
        assert_eq!(0, x.len);
    }

    // Scenario: Overlap, request equal
    #[test]
    fn test_read_10() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![5, 6, 7, 0, 0, 0, 0, 1, 2, 3, 4].into_boxed_slice();
        x.off = 7;
        x.len = 7;
        let mut b = [9, 9, 9, 9, 9, 9, 9, 9];
        assert_eq!(7, x.read(&mut b));
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 9], b);
        assert_eq!(0, x.off);
        assert_eq!(0, x.len);
    }

    // Scenario: No overlap
    #[test]
    fn test_grow_01() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![9, 9, 9, 1, 2, 3, 9, 9].into_boxed_slice();
        x.off = 3;
        x.len = 3;
        x.grow(1);
        assert_eq!([1, 2, 3, 0, 0, 0, 0, 0, 0][..], x.buf[..]);
        assert_eq!(0, x.off);
        assert_eq!(3, x.len);
    }

    // Scenario: Overlap
    #[test]
    fn test_grow_02() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![5, 6, 7, 9, 9, 9, 9, 1, 2, 3, 4].into_boxed_slice();
        x.off = 7;
        x.len = 7;
        x.grow(1);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0][..], x.buf[..]);
        assert_eq!(0, x.off);
        assert_eq!(7, x.len);
    }

    // Scenario: Passed buffer is empty
    #[test]
    fn test_write_01() {
        let mut x = RingBuffer::new(32);
        x.buf = vec![1].into_boxed_slice();
        x.len = 1;
        let b = [];
        assert_eq!(0, x.write(&b));
    }

    // Scenario: Internal buffer is full
    #[test]
    fn test_write_02() {
        let mut x = RingBuffer::new(0);
        let b = [1];
        assert_eq!(0, x.write(&b));
    }

    // Scenario: Buffer resize required (below max capacity)
    #[test]
    fn test_write_03() {
        let mut x = RingBuffer::new(32);
        let b = [1, 2, 3];
        assert_eq!(3, x.write(&b));
        assert_eq!(0, x.off);
        assert_eq!(3, x.len);
        assert_eq!(6, x.buf.len());
    }

    // Scenario: Buffer resize required (hits max capacity)
    #[test]
    fn test_write_04() {
        let mut x = RingBuffer::new(5);
        let b = [1, 2, 3];
        assert_eq!(3, x.write(&b));
        assert_eq!(0, x.off);
        assert_eq!(3, x.len);
        assert_eq!(5, x.buf.len());
    }

    // Scenario: Overlap
    #[test]
    fn test_write_05() {
        let mut x = RingBuffer::new(6);
        x.buf = vec![9, 9, 9, 1, 2, 9].into_boxed_slice();
        x.off = 3;
        x.len = 2;
        let b = [3, 4, 5];
        assert_eq!(3, x.write(&b));
        assert_eq!(&[4, 5, 9, 1, 2, 3][..], &x.buf[..]);
        assert_eq!(3, x.off);
        assert_eq!(5, x.len);
    }
}
