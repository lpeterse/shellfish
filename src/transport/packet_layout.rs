use std::ops::Range;

#[derive(Debug)]
pub struct Packet<T> {
    padding: u8,
    payload: T,
}

#[derive(Debug)]
pub struct PacketLayout {
    pub padding_len: usize,
    pub payload_len: usize,
    pub mac_len: usize,
}

impl PacketLayout {
    pub const MAX_PACKET_LEN: usize = 35000;
    pub const PACKET_LEN_SIZE: usize = 4;
    pub const PACKET_MIN_SIZE: usize = 16;
    pub const PADDING_LEN_SIZE: usize = 1;
    pub const PADDING_MIN_SIZE: usize = 4;
    pub const PAYLOAD_OFFSET: usize = 5;

    pub fn new(payload_len: usize, block_len: usize, mac_len: usize) -> PacketLayout {
        let padding_len = {
            let l = Self::PACKET_LEN_SIZE + Self::PADDING_LEN_SIZE + payload_len;
            let mut p = block_len - (l % block_len);
            if p < Self::PADDING_MIN_SIZE {
                p += block_len
            };
            while p + l < Self::PACKET_MIN_SIZE {
                p += block_len
            }
            p
        };
        PacketLayout {
            padding_len,
            payload_len,
            mac_len,
        }
    }

    pub fn new_aad(payload_len: usize, block_len: usize, mac_len: usize) -> PacketLayout {
        let padding_len = {
            let l = Self::PADDING_LEN_SIZE + payload_len;
            let mut p = block_len - (l % block_len);
            if p < Self::PADDING_MIN_SIZE {
                p += block_len
            };
            while p + l < Self::PACKET_MIN_SIZE {
                p += block_len
            }
            p
        };
        PacketLayout {
            padding_len,
            payload_len,
            mac_len,
        }
    }

    pub fn buffer_len(&self) -> usize {
        Self::PACKET_LEN_SIZE
            + Self::PADDING_LEN_SIZE
            + self.payload_len
            + self.padding_len
            + self.mac_len
    }

    pub fn packet_len(&self) -> usize {
        Self::PADDING_LEN_SIZE + self.payload_len + self.padding_len
    }

    pub fn packet_len_range(&self) -> Range<usize> {
        Range {
            start: 0,
            end: Self::PACKET_LEN_SIZE,
        }
    }

    pub fn padding_len_range(&self) -> Range<usize> {
        Range {
            start: Self::PACKET_LEN_SIZE,
            end: Self::PACKET_LEN_SIZE + Self::PADDING_LEN_SIZE,
        }
    }

    pub fn payload_range(&self) -> Range<usize> {
        Range {
            start: Self::PACKET_LEN_SIZE + Self::PADDING_LEN_SIZE,
            end: Self::PACKET_LEN_SIZE + Self::PADDING_LEN_SIZE + self.payload_len,
        }
    }

    pub fn padding_range(&self) -> Range<usize> {
        Range {
            start: Self::PACKET_LEN_SIZE + Self::PADDING_LEN_SIZE + self.payload_len,
            end: Self::PACKET_LEN_SIZE
                + Self::PADDING_LEN_SIZE
                + self.payload_len
                + self.padding_len,
        }
    }

    pub fn integrity_range(&self) -> Range<usize> {
        Range {
            start: 0,
            end: Self::PACKET_LEN_SIZE
                + Self::PADDING_LEN_SIZE
                + self.payload_len
                + self.padding_len,
        }
    }

    pub fn cipher_range(&self) -> Range<usize> {
        Range {
            start: Self::PACKET_LEN_SIZE,
            end: Self::PACKET_LEN_SIZE
                + Self::PADDING_LEN_SIZE
                + self.payload_len
                + self.padding_len,
        }
    }

    pub fn mac_range(&self) -> Range<usize> {
        Range {
            start: Self::PACKET_LEN_SIZE
                + Self::PADDING_LEN_SIZE
                + self.payload_len
                + self.padding_len,
            end: Self::PACKET_LEN_SIZE
                + Self::PADDING_LEN_SIZE
                + self.payload_len
                + self.padding_len
                + self.mac_len,
        }
    }

    pub fn pad_zero(&self, buf: &mut [u8]) {
        buf[Self::PACKET_LEN_SIZE] = self.padding_len as u8;
        for i in &mut buf[self.padding_range()] {
            *i = 0
        }
    }
    
    pub fn put_len(&self, buf: &mut [u8]) {
        let packet_len = (self.packet_len() as u32).to_be_bytes();
        buf[self.packet_len_range()].copy_from_slice(&packet_len);
    }
}
