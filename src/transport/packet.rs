use crate::codec::*;

pub struct Packet<'a, T> {
    buffer_len: usize,
    padding_len: usize,
    payload_len: usize,
    payload: &'a T,
}

impl<'a, T: Encode> Packet<'a, T> {
    pub const MAX_PACKET_LEN: usize = 35000;
    pub const PACKET_LEN_LEN: usize = 4;
    pub const PACKET_MIN_LEN: usize = 16;
    pub const PADDING_LEN_LEN: usize = 1;
    pub const PADDING_MIN_LEN: usize = 4;

    pub fn new(aad: bool, block_len: usize, mac_len: usize, payload: &'a T) -> Self {
        let payload_len = payload.size();
        let padding_len = {
            let l =
                if aad { 0 } else { Self::PACKET_LEN_LEN } + Self::PADDING_LEN_LEN + payload_len;
            let mut p = block_len - (l % block_len);
            if p < Self::PADDING_MIN_LEN {
                p += block_len
            };
            while p + l < Self::PACKET_MIN_LEN {
                p += block_len
            }
            p
        };
        let buffer_len =
            Self::PACKET_LEN_LEN + Self::PADDING_LEN_LEN + payload_len + padding_len + mac_len;
        Self {
            buffer_len,
            padding_len,
            payload_len,
            payload,
        }
    }
}

impl<'a, T: Encode> Encode for Packet<'a, T> {
    fn size(&self) -> usize {
        // Buffer len is precomputed (as is padding_len).
        self.buffer_len
    }

    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be((1 + self.payload_len + self.padding_len) as u32);
        e.push_u8(self.padding_len as u8);
        Encode::encode(self.payload, e);
        std::iter::repeat(())
            .take(self.padding_len)
            .for_each(|()| e.push_u8(0));
        // MAC area stays uninitialised. It's the cipher's responsibility to initialize it.
    }
}
