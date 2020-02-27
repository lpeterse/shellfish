use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgChannelClose {
    pub recipient_channel: u32,
}

impl Message for MsgChannelClose {
    const NUMBER: u8 = 97;
}

impl  Encode for MsgChannelClose {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        e.push_u32be(self.recipient_channel);
    }
}

impl Decode for MsgChannelClose {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
        }.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelClose { recipient_channel: 23 };
        assert_eq!(
            "MsgChannelClose { recipient_channel: 23 }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelClose { recipient_channel: 23 };
        assert_eq!(&[97,0,0,0,23][..], &BEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [97,0,0,0,23];
        let msg: MsgChannelClose = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
    }
}
