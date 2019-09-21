use std::ops::Range;

pub struct Token {
    packet_counter: u64,
    buffer_range: Range<usize>,
}

impl Token {
    pub fn new(packet_counter: u64, buffer_range: Range<usize>) -> Self {
        Self { packet_counter, buffer_range }
    }
}
