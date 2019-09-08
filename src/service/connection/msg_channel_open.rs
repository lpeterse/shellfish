pub use super::channel_type::*;

use crate::codec::*;

#[derive(Clone, Debug)]
pub struct MsgChannelOpen<'a,T: ChannelType<'a>> {
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub channel_type: T::Open,
}

impl<'a,T: ChannelType<'a>> MsgChannelOpen<'a,T> {
    const MSG_NUMBER: u8 = 90;
}

impl<'a,T: ChannelType<'a>> Codec<'a> for MsgChannelOpen<'a,T> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4
        + Codec::size(&self.channel_type.name())
        + Codec::size(&self.channel_type)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.channel_type.name(), e);
        e.push_u32be(self.sender_channel);
        e.push_u32be(self.initial_window_size);
        e.push_u32be(self.maximum_packet_size);
        Codec::encode(&self.channel_type, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        let name: &str = Codec::decode(d)?;
        Some(Self {
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            channel_type: Named::decode(d, name)?,
        })
    }
}
