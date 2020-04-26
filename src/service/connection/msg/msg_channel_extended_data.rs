use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub(crate) struct MsgChannelExtendedData<'a> {
    pub recipient_channel: u32,
    pub data_type_code: u32,
    pub data: &'a [u8],
}

impl<'a> MsgChannelExtendedData<'a> {
    pub fn new(recipient_channel: u32, data_type_code: u32, data: &'a [u8]) -> Self {
        Self {
            recipient_channel,
            data_type_code,
            data,
        }
    }
}

impl<'a> Message for MsgChannelExtendedData<'a> {
    const NUMBER: u8 = 95;
}

impl<'a> Encode for MsgChannelExtendedData<'a> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.data_type_code);
        e.push_u32be(self.data.len() as u32);
        e.push_bytes(&self.data);
    }
}

impl<'a> DecodeRef<'a> for MsgChannelExtendedData<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let recipient_channel = d.take_u32be()?;
        let data_type_code = d.take_u32be()?;
        let len = d.take_u32be()?;
        let data = d.take_bytes(len as usize)?;
        Self {
            recipient_channel,
            data_type_code,
            data,
        }
        .into()
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
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 16] = [95, 0, 0, 0, 23, 0, 0, 0, 4, 0, 0, 0, 3, 1, 2, 3];
        let msg: MsgChannelExtendedData = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
        assert_eq!(msg.data_type_code, 4);
        assert_eq!(msg.data, [1, 2, 3]);
    }
}
