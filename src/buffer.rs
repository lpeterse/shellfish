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
    ///
    /// `self.len() == self.as_ref().len()`
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Returns `true` if the window has a length of `0`.
    pub fn is_empty(&self) -> bool {
        // Invariant: Empty window is always reset to the left
        self.window.end == 0
    }

    /// Returns the underlying buffer length.
    ///
    /// `self.len() <= self.capacity()`
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// The number of bytes right of the window.
    pub fn available(&mut self) -> usize {
        self.buffer.len() - self.window.end
    }

    /// The slice right of the window.
    pub fn available_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.window.end..]
    }

    /// Shrink the window from the left by `len` bytes.
    ///
    /// Resets the window position to 0 when the window becomes empty.
    pub fn consume(&mut self, len: usize) {
        assert!(self.window.start + len <= self.buffer.len());
        self.window.start += len;
        if self.window.start == self.window.end {
            self.window.start = 0;
            self.window.end = 0;
        }
    }

    /// Extend the window to the right and return the extension area.
    pub fn extend(&mut self, len: usize) -> &mut [u8] {
        assert!(self.window.end + len <= self.buffer.len());
        let start = self.window.end;
        self.window.end += len;
        &mut self.buffer.as_mut()[start..self.window.end]
    }

    /// Move the window to the leftmost position by copying the data.
    ///
    /// Is idempotent and does nothing when the left window position is already 0.
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
    /// Example: `self.increase_capacity(2 * self.capacity())`
    pub fn increase_capacity(&mut self, capacity: usize) {
        if capacity > self.buffer.len() {
            let len = self.window.len();
            let mut new = Vec::with_capacity(capacity);
            new.resize(capacity, 0);
            new[..len].copy_from_slice(&self.buffer[self.window.clone()]);
            self.buffer = new.into_boxed_slice();
            self.window.end = len;
            self.window.start = 0;
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let len = std::cmp::min(self.len(), buf.len());
        buf[..len].copy_from_slice(&self.as_ref()[..len]);
        self.consume(len);
        len
    }

    /*
    pub fn write(&mut self, buf: &[u8]) -> usize {
        let len = std::cmp::min(buf.len(), self.capacity() - self.window.len());
        self.write_all(&buf[..len]);
        len
    }*/

    pub fn write_all(&mut self, buf: &[u8]) {
        if self.available() < buf.len() {
            let required_capacity = self.len() + buf.len();
            if required_capacity <= self.capacity() {
                self.pushback()
            } else {
                self.increase_capacity(required_capacity)
            }
        }
        self.extend(buf.len()).copy_from_slice(buf);
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.buffer[self.window.start..self.window.end]
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.window.start..self.window.end]
    }
}
