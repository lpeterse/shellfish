use std::ops::Range;

/// A specialised buffer implementation.
///
/// The buffer maintains a window that represents the part of the buffer that holds actual data.
/// The window can be extended to the right by calling `extend` and shrinked from the left by
/// calling `consume`. The window area can be accessed using the `AsRef` and `AsMut` instances.
///
/// After consuming the whole buffer (window empty), the window is automatically reset to the
/// leftmost position. When the window is non-empty, `pushback` can be used to copy all data to
/// the left. The first scenario is very cheap and assumed to be very likely.
///
/// The window may only be extended from the right window position to the buffer end position.
/// In case more space is required, you should try the following in this order:
///
///   - Consume the buffer before writing to it again (if possible and applicable)
///   - Use `pushback` (this keeps memory constant and does not cause a re-allocation)
///   - Use `increase_capacity` (this increases memory consumption and costs an allocation)
#[derive(Debug)]
pub struct Buffer {
    buffer: Box<[u8]>,
    window: Range<usize>,
}

impl Buffer {
    /// Create a new buffer with given initial capacity.
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize(capacity, 0);
        let buffer = buffer.into_boxed_slice();
        let window = Range { start: 0, end: 0 };
        Self { buffer, window }
    }

    /// Return the window length.
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Returns `true` iff the window has a length of `0`.
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Returns the underlying buffer length.
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// The number of bytes right of the window.
    pub fn available(&mut self) -> usize {
        self.buffer.len() - self.window.end
    }

    /// The slice right of the window.
    pub fn available_mut(&mut self) -> &mut [u8] {
        self.buffer.get_mut(self.window.end..).unwrap_or(&mut [])
    }

    /// Shrink the window from the left by `len` bytes.
    ///
    /// Resets the window position to 0 when the window becomes empty.
    ///
    /// Panics if `len` exceeds window length.
    pub fn consume(&mut self, len: usize) {
        assert!(len <= self.window.len());
        self.window.start += len;
        if self.window.start == self.window.end {
            self.window.start = 0;
            self.window.end = 0;
        }
    }

    /// Extend the window to the right.
    ///
    /// Panics if resulting window would exceed buffer length.
    pub fn extend(&mut self, len: usize) {
        assert!(self.window.end + len <= self.buffer.len());
        self.window.end += len;
    }

    /// Move the window to the leftmost position by copying the data.
    ///
    /// Is idempotent and does nothing if the left window position is already 0.
    pub fn pushback(&mut self) {
        if self.window.start != 0 {
            self.buffer.copy_within(self.window.clone(), 0);
            self.window.end = self.window.len();
            self.window.start = 0;
        }
    }

    /// Increase the capacity to new value.
    ///
    /// Allocates a new piece of boxed memory, copies the old data to it and then drops the old
    /// memory. The new window starts at position 0, so this includes an implicit `pushback`.
    ///
    /// Panics if given capacity is smaller or equal than current capacity.
    ///
    /// Example: `self.increase_capacity(2 * self.capacity())`
    pub fn increase_capacity(&mut self, capacity: usize) {
        assert!(capacity > self.capacity());
        let len = self.window.len();
        let mut new = Vec::with_capacity(capacity);
        new.resize(capacity, 0);
        new[..len].copy_from_slice(self.as_ref());
        self.buffer = new.into_boxed_slice();
        self.window.end = len;
        self.window.start = 0;
    }

    /// Read into supplied buffer and return the number of bytes read.
    ///
    /// The bytes read by this function are consumed automatically.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let len = std::cmp::min(self.len(), buf.len());
        buf[..len].copy_from_slice(&self.as_ref()[..len]);
        self.consume(len);
        len
    }

    /// Write all bytes from supplied buffer.
    ///
    /// Perform pushback / capacity increase if necessary.
    pub fn write_all(&mut self, buf: &[u8]) {
        if self.available() < buf.len() {
            let required_capacity = self.len() + buf.len();
            if required_capacity <= self.capacity() {
                self.pushback()
            } else {
                self.increase_capacity(required_capacity)
            }
        }
        self.available_mut()[..buf.len()].copy_from_slice(buf);
        self.extend(buf.len());
    }
}

impl AsRef<[u8]> for Buffer {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.buffer
            .get(self.window.start..self.window.end)
            .unwrap_or(&[])
    }
}

impl AsMut<[u8]> for Buffer {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        self.buffer
            .get_mut(self.window.start..self.window.end)
            .unwrap_or(&mut [])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test properties of a buffer with empty capacity.
    #[test]
    fn new_with_empty_capacity() {
        let buf = Buffer::new(0);

        assert_eq!(buf.buffer.len(), 0);
        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 0);
    }

    /// Test properties of a buffer with non-empty capacity.
    #[test]
    fn new_with_some_capacity() {
        let buf = Buffer::new(123);

        assert_eq!(buf.buffer.len(), 123);
        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 0);
    }

    /// Test the methods `len()` and `is_empty()` for empty window.
    #[test]
    fn len_and_is_empty_for_empty_window() {
        let buf = Buffer::new(0);

        assert_eq!(buf.is_empty(), true);
        assert_eq!(buf.len(), 0);
    }

    /// Test the methods `len()` and `is_empty()` for non-empty window.
    #[test]
    fn len_and_is_empty_for_non_empty_window() {
        let mut buf = Buffer::new(0);
        buf.window.end = 100;
        buf.window.start = 10;

        assert_eq!(buf.is_empty(), false);
        assert_eq!(buf.len(), 90);
    }

    /// Test the methods `available()` and `available_mut()` for full buffer.
    #[test]
    fn available_for_full_buffer() {
        let mut buf = Buffer::new(100);
        buf.window.end = 100;

        assert_eq!(buf.available(), 0);
        assert_eq!(buf.available_mut(), []);
    }

    /// Test the methods `available()` and `available_mut()` for non-full buffer.
    #[test]
    fn available_for_non_full_buffer() {
        let mut buf = Buffer::new(100);
        buf.window.end = 98;
        buf.buffer[98] = 3;
        buf.buffer[99] = 4;

        assert_eq!(buf.available(), 2);
        assert_eq!(buf.available_mut(), [3, 4]);
    }

    /// Test consuming some of the window.
    #[test]
    fn consume_some() {
        let mut buf = Buffer::new(100);
        buf.window.start = 40;
        buf.window.end = 60;
        buf.consume(10);

        assert_eq!(buf.window.start, 50);
        assert_eq!(buf.window.end, 60);
    }

    /// Test consuming all of the window.
    #[test]
    fn consume_all() {
        let mut buf = Buffer::new(100);
        buf.window.start = 40;
        buf.window.end = 60;
        buf.consume(20);

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 0);
    }

    /// Test extending the buffer to maximum.
    #[test]
    fn extend_max() {
        let mut buf = Buffer::new(100);
        buf.window.start = 40;
        buf.window.end = 50;
        buf.extend(50);

        assert_eq!(buf.window.start, 40);
        assert_eq!(buf.window.end, 100);
    }

    /// Tests that a pushback does nothing when window is already left-aligned.
    #[test]
    fn pushback_noop() {
        let mut buf = Buffer::new(100);
        buf.window.end = 50;
        buf.pushback();

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 50);
    }

    /// Tests that a pushback moves overlapping data correctly.
    #[test]
    fn pushback_overlap() {
        let mut buf = Buffer::new(100);
        buf.window.start = 2;
        buf.window.end = 5;
        buf.buffer[0] = 255;
        buf.buffer[1] = 1;
        buf.buffer[2] = 2;
        buf.buffer[3] = 3;
        buf.buffer[4] = 4;
        buf.buffer[5] = 5;
        buf.buffer[6] = 6;
        buf.pushback();

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 3);
        assert_eq!(buf.buffer[0], 2);
        assert_eq!(buf.buffer[1], 3);
        assert_eq!(buf.buffer[2], 4);
        assert_eq!(buf.buffer[3], 3);
    }

    /// Tests increasing capacity for a buffer with not-left-aligned-data.
    #[test]
    fn increase_capacity() {
        let mut buf = Buffer::new(10);
        buf.window.start = 2;
        buf.window.end = 4;
        buf.buffer[2] = 2;
        buf.buffer[3] = 3;
        buf.increase_capacity(11);

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 2);
        assert_eq!(buf.buffer[0], 2);
        assert_eq!(buf.buffer[1], 3);
        assert_eq!(buf.buffer.len(), 11);
    }

    /// Test reading into empty buffer.
    #[test]
    fn read_empty() {
        let mut buf = Buffer::new(10);
        buf.buffer[2] = 2;
        buf.buffer[3] = 3;
        buf.window.start = 2;
        buf.window.end = 4;
        let mut dst = [];
        let read = buf.read(dst.as_mut());

        assert_eq!(read, 0);
        assert_eq!(buf.window.start, 2);
        assert_eq!(buf.window.end, 4);
    }

    /// Test reading into smaller buffer.
    #[test]
    fn read_smaller() {
        let mut buf = Buffer::new(10);
        buf.buffer[2] = 2;
        buf.buffer[3] = 3;
        buf.window.start = 2;
        buf.window.end = 4;
        let mut dst = [0u8];
        let read = buf.read(dst.as_mut());

        assert_eq!(read, 1);
        assert_eq!(dst[0], 2);
        assert_eq!(buf.window.start, 3);
        assert_eq!(buf.window.end, 4);
    }

    /// Test reading into larger buffer.
    #[test]
    fn read_larger() {
        let mut buf = Buffer::new(10);
        buf.buffer[2] = 2;
        buf.buffer[3] = 3;
        buf.window.start = 2;
        buf.window.end = 4;
        let mut dst = [0, 0, 0u8];
        let read = buf.read(dst.as_mut());

        assert_eq!(read, 2);
        assert_eq!(dst[0], 2);
        assert_eq!(dst[1], 3);
        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 0);
    }

    /// Test writing all into buffer with sufficient available space.
    #[test]
    fn write_all_sufficient_available() {
        let mut buf = Buffer::new(10);
        buf.window.start = 0;
        buf.window.end = 3;
        let src = [1, 2, 3, 4, 5, 6, 7u8];
        buf.write_all(src.as_ref());

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 10);
        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.as_ref()[3], 1);
        assert_eq!(buf.as_ref()[9], 7);
    }

    /// Test writing all into buffer with insufficient available space.
    #[test]
    fn write_all_insufficient_available() {
        let mut buf = Buffer::new(10);
        buf.window.start = 1;
        buf.window.end = 4;
        buf.buffer[1] = 255;
        buf.buffer[2] = 254;
        buf.buffer[3] = 253;
        let src = [1, 2, 3, 4, 5, 6, 7u8];
        buf.write_all(src.as_ref());

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 10);
        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.as_ref()[0], 255);
        assert_eq!(buf.as_ref()[1], 254);
        assert_eq!(buf.as_ref()[2], 253);
        assert_eq!(buf.as_ref()[3], 1);
        assert_eq!(buf.as_ref()[9], 7);
    }

    /// Test writing all into buffer with insufficient capacity.
    #[test]
    fn write_all_insufficient_capacity() {
        let mut buf = Buffer::new(9);
        buf.window.start = 1;
        buf.window.end = 4;
        buf.buffer[1] = 255;
        buf.buffer[2] = 254;
        buf.buffer[3] = 253;
        let src = [1, 2, 3, 4, 5, 6, 7u8];
        buf.write_all(src.as_ref());

        assert_eq!(buf.window.start, 0);
        assert_eq!(buf.window.end, 10);
        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.as_ref()[0], 255);
        assert_eq!(buf.as_ref()[1], 254);
        assert_eq!(buf.as_ref()[2], 253);
        assert_eq!(buf.as_ref()[3], 1);
        assert_eq!(buf.as_ref()[9], 7);
    }
}
