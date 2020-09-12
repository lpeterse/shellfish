use crate::util::codec::*;
use crate::transport::Message;

#[derive(Debug)]
pub(crate) struct MsgChannelEof {
    pub recipient_channel: u32,
}

impl MsgChannelEof {
    pub fn new(recipient_channel: u32) -> Self {
        Self { recipient_channel }
    }
}

impl Message for MsgChannelEof {
    const NUMBER: u8 = 96;
}

impl Encode for MsgChannelEof {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)
    }
}

impl Decode for MsgChannelEof {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelEof {
            recipient_channel: 23,
        };
        assert_eq!(
            "MsgChannelEof { recipient_channel: 23 }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelEof {
            recipient_channel: 23,
        };
        assert_eq!(&[96, 0, 0, 0, 23][..], &SliceEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [96, 0, 0, 0, 23];
        let msg: MsgChannelEof = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
    }
}
