use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgChannelExtendedData<'a> {
    pub recipient_channel: u32,
    pub data_type_code: u32,
    pub data: &'a [u8],
}

impl<'a> Message for MsgChannelExtendedData<'a> {
    const NUMBER: u8 = 95;
}

impl<'a> SshEncode for MsgChannelExtendedData<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_u32be(self.data_type_code)?;
        e.push_bytes_framed(&self.data)
    }
}

impl<'a> SshDecodeRef<'a> for MsgChannelExtendedData<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            recipient_channel: d.take_u32be()?,
            data_type_code: d.take_u32be()?,
            data: d.take_bytes_framed()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelExtendedData {
            recipient_channel: 23,
            data_type_code: 4,
            data: &[1, 2, 3],
        };
        assert_eq!(
            "MsgChannelExtendedData { recipient_channel: 23, data_type_code: 4, data: [1, 2, 3] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelExtendedData {
            recipient_channel: 23,
            data_type_code: 4,
            data: &[1, 2, 3],
        };
        assert_eq!(
            &[95, 0, 0, 0, 23, 0, 0, 0, 4, 0, 0, 0, 3, 1, 2, 3][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 16] = [95, 0, 0, 0, 23, 0, 0, 0, 4, 0, 0, 0, 3, 1, 2, 3];
        let msg: MsgChannelExtendedData = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
        assert_eq!(msg.data_type_code, 4);
        assert_eq!(msg.data, [1, 2, 3]);
    }
}
