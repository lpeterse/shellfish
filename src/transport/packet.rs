use crate::codec_ssh::*;
use crate::codec::*;

pub struct Packet<T> {
    payload: T
}

impl <T> Packet<T> {
    pub fn new(payload: T) -> Self {
        Self { payload }
    }
}

impl <'a, T: SshCodec<'a>> SshCodec<'a> for Packet<T> {
    fn size(&self) -> usize {
        let s = self.payload.size();
        println!("SSS {}", 1 + s + padding_by_payload_size(s));
        1 + s + padding_by_payload_size(s)
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        let padding = padding_by_payload_size(self.payload.size());
        c.push_u8(padding as u8);
        SshCodec::encode(&self.payload, c);
        for _ in 1..padding {
            c.push_u8(0);
        }
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let padding = c.take_u8()?;
        let payload =  SshCodec::decode(c)?;
        Some(Packet { payload })
    }
}

fn padding_by_payload_size(payload_size: usize) -> usize {
    const BLOCK_SIZE: usize = 16;
    const PADDING_MIN_SIZE: usize = 4;
    const PACKET_LEN_SIZE: usize = 4;
    const PADDING_LEN_SIZE: usize = 1;

    let len = PACKET_LEN_SIZE + PADDING_LEN_SIZE + payload_size;
    let padding = BLOCK_SIZE - (len % BLOCK_SIZE);

    if padding < PADDING_MIN_SIZE {
        padding + BLOCK_SIZE
    } else {
        padding
    }
}
