use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgChannelData<'a> {
    pub recipient_channel: u32,
    pub data: &'a [u8],
}

impl<'a> MsgChannelData<'a> {
    pub fn new(recipient_channel: u32, data: &'a [u8]) -> Self {
        Self {
            recipient_channel,
            data,
        }
    }
}

impl<'a> Message for MsgChannelData<'a> {
    const NUMBER: u8 = 94;
}

impl<'a> SshEncode for MsgChannelData<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_bytes_framed(&self.data)
    }
}

impl<'a> SshDecodeRef<'a> for MsgChannelData<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            recipient_channel: d.take_u32be()?,
            data: d.take_bytes_framed()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelData {
            recipient_channel: 23,
            data: &[1, 2, 3],
        };
        assert_eq!(
            "MsgChannelData { recipient_channel: 23, data: [1, 2, 3] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelData {
            recipient_channel: 23,
            data: &[1, 2, 3],
        };
        assert_eq!(
            &[94, 0, 0, 0, 23, 0, 0, 0, 3, 1, 2, 3][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 12] = [94, 0, 0, 0, 23, 0, 0, 0, 3, 1, 2, 3];
        let msg: MsgChannelData = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
        assert_eq!(msg.data, [1, 2, 3]);
    }
}
