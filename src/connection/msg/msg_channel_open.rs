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

impl<S: AsRef<str>> MsgChannelOpen<S> {
    pub fn new(name: S, id: u32, ws: u32, ps: u32, data: Vec<u8>) -> Self {
        Self {
            name,
            sender_channel: id,
            initial_window_size: ws,
            maximum_packet_size: ps,
            data,
        }
    }
}

impl<S: AsRef<str>> Message for MsgChannelOpen<S> {
    const NUMBER: u8 = 90;
}

impl Encode for MsgChannelOpen<&'static str> {
    fn size(&self) -> usize {
        17 + self.name.len() + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push_encode(&self.name)?;
        e.push_u32be(self.sender_channel)?;
        e.push_u32be(self.initial_window_size)?;
        e.push_u32be(self.maximum_packet_size)?;
        e.push_bytes(&self.data)
    }
}

impl Decode for MsgChannelOpen {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            name: Decode::decode(d)?,
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            data: d.take_all()?.into(),
        }
        .into()
    }
}
