use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgChannelSuccess {
    pub recipient_channel: u32,
}

impl MsgChannelSuccess {
    pub fn new(recipient_channel: u32) -> Self {
        Self { recipient_channel }
    }
}

impl Message for MsgChannelSuccess {
    const NUMBER: u8 = 99;
}

impl SshEncode for MsgChannelSuccess {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)
    }
}

impl SshDecode for MsgChannelSuccess {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            recipient_channel: d.take_u32be()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelSuccess {
            recipient_channel: 23,
        };
        assert_eq!(
            "MsgChannelSuccess { recipient_channel: 23 }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelSuccess {
            recipient_channel: 23,
        };
        assert_eq!(&[99, 0, 0, 0, 23][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [99, 0, 0, 0, 23];
        let msg: MsgChannelSuccess = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
    }
}
