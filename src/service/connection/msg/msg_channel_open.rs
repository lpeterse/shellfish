use super::*;
use crate::message::*;

#[derive(Debug)]
pub(crate) struct MsgChannelOpen<T: Channel> {
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub channel_type: T::Open,
}

impl<'a, T: Channel> Message for MsgChannelOpen<T> {
    const NUMBER: u8 = 90;
}

impl<T: Channel> Encode for MsgChannelOpen<T> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4 + Encode::size(&<T as Channel>::NAME) + Encode::size(&self.channel_type)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        Encode::encode(&<T as Channel>::NAME, e);
        e.push_u32be(self.sender_channel);
        e.push_u32be(self.initial_window_size);
        e.push_u32be(self.maximum_packet_size);
        Encode::encode(&self.channel_type, e);
    }
}

impl<T: Channel> Decode for MsgChannelOpen<T> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let _: &str = DecodeRef::decode(d).filter(|x| x == &<T as Channel>::NAME)?;
        Self {
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            channel_type: Decode::decode(d)?,
        }
        .into()
    }
}
