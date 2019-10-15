use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub struct MsgChannelSuccess {
    pub recipient_channel: u32,
}

impl Message for MsgChannelSuccess {
    const NUMBER: u8 = 99;
}

impl Encode for MsgChannelSuccess {
    fn size(&self) -> usize {
        1 + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        e.push_u32be(self.recipient_channel);
    }
}

impl Decode for MsgChannelSuccess {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?
        }.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelSuccess { recipient_channel: 23 };
        assert_eq!(
            "MsgChannelSuccess { recipient_channel: 23 }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelSuccess { recipient_channel: 23 };
        assert_eq!(&[99,0,0,0,23][..], &BEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [99,0,0,0,23];
        let msg: MsgChannelSuccess = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
    }
}
