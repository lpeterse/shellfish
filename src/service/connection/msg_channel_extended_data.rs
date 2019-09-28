use crate::codec::*;

#[derive(Debug)]
pub struct MsgChannelExtendedData<'a> {
    pub recipient_channel: u32,
    pub data_type_code: u32,
    pub data: &'a [u8],
}

impl<'a> MsgChannelExtendedData<'a> {
    const MSG_NUMBER: u8 = 95;
}

impl<'a> Encode for MsgChannelExtendedData<'a> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.data_type_code);
        e.push_u32be(self.data.len() as u32);
        e.push_bytes(&self.data);
    }
}

impl <'a> DecodeRef<'a> for MsgChannelExtendedData<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
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
