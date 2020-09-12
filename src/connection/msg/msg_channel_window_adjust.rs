use crate::util::codec::*;
use crate::transport::Message;

#[derive(Clone, Debug)]
pub(crate) struct MsgChannelWindowAdjust {
    pub recipient_channel: u32,
    pub bytes_to_add: u32,
}

impl MsgChannelWindowAdjust {
    pub fn new(recipient_channel: u32, bytes_to_add: u32) -> Self {
        Self {
            recipient_channel,
            bytes_to_add,
        }
    }
}

impl Message for MsgChannelWindowAdjust {
    const NUMBER: u8 = 93;
}

impl Encode for MsgChannelWindowAdjust {
    fn size(&self) -> usize {
        1 + 4 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E)-> Option<()>  {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_u32be(self.bytes_to_add)
    }
}

impl Decode for MsgChannelWindowAdjust {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        MsgChannelWindowAdjust {
            recipient_channel: d.take_u32be()?,
            bytes_to_add: d.take_u32be()?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelWindowAdjust {
            recipient_channel: 23,
            bytes_to_add: 47,
        };
        assert_eq!(
            "MsgChannelWindowAdjust { recipient_channel: 23, bytes_to_add: 47 }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelWindowAdjust {
            recipient_channel: 23,
            bytes_to_add: 47,
        };
        assert_eq!(
            &[93, 0, 0, 0, 23, 0, 0, 0, 47][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 9] = [93, 0, 0, 0, 23, 0, 0, 0, 47];
        let msg: MsgChannelWindowAdjust = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
        assert_eq!(msg.bytes_to_add, 47);
    }
}
