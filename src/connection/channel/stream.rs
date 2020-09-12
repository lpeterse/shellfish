use crate::util::buffer::Buffer;

#[derive(Debug)]
pub struct Stream {
    pub rx: Buffer,
    pub tx: Buffer,
}

impl Default for Stream {
    fn default() -> Self {
        Self {
            rx: Buffer::new(0),
            tx: Buffer::new(0),
        }
    }
}
