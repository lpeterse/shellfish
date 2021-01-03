use super::Message;
use crate::util::codec::*;

#[derive(Copy, Clone, Debug)]
pub struct MsgUnimplemented {
    pub packet_number: u32,
}

impl Message for MsgUnimplemented {
    const NUMBER: u8 = 3;
}

impl SshEncode for MsgUnimplemented {
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(<Self as Message>::NUMBER)?;
        c.push_u32be(self.packet_number)
    }
}

impl SshDecode for MsgUnimplemented {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            packet_number: d.take_u32be()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgUnimplemented { packet_number: 23 };
        assert_eq!(&[3, 0, 0, 0, 23][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [3, 0, 0, 0, 23];
        let msg: MsgUnimplemented = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(23, msg.packet_number);
    }
}
