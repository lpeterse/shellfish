use crate::util::codec::*;
use crate::transport::Message;

#[derive(Debug)]
pub(crate) struct MsgChannelFailure {
    pub recipient_channel: u32,
}

impl Message for MsgChannelFailure {
    const NUMBER: u8 = 100;
}

impl Encode for MsgChannelFailure {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)
    }
}

impl Decode for MsgChannelFailure {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?
        }.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelFailure { recipient_channel: 23 };
        assert_eq!(
            "MsgChannelFailure { recipient_channel: 23 }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelFailure { recipient_channel: 23 };
        assert_eq!(&[100,0,0,0,23][..], &SliceEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [100,0,0,0,23];
        let msg: MsgChannelFailure = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
    }
}
