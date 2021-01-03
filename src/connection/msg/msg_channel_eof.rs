use crate::transport::Message;
use crate::util::codec::*;

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

impl SshEncode for MsgChannelEof {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)
    }
}

impl SshDecode for MsgChannelEof {
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
        assert_eq!(&[96, 0, 0, 0, 23][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [96, 0, 0, 0, 23];
        let msg: MsgChannelEof = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
    }
}
