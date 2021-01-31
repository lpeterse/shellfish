use super::super::*;
use crate::transport::Message;

#[derive(Debug)]
pub(crate) struct MsgChannelOpen<S: AsRef<str> = String> {
    pub name: S,
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub data: Vec<u8>,
}

impl<S: AsRef<str>> Message for MsgChannelOpen<S> {
    const NUMBER: u8 = 90;
}

impl SshEncode for MsgChannelOpen<&'static str> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_str_framed(&self.name)?;
        e.push_u32be(self.sender_channel)?;
        e.push_u32be(self.initial_window_size)?;
        e.push_u32be(self.maximum_packet_size)?;
        e.push_bytes(&self.data)
    }
}

impl SshDecode for MsgChannelOpen {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            name: d.take_str_framed()?.into(),
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            data: d.take_bytes_all()?.into(),
        }
        .into()
    }
}
