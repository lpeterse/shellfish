use super::*;
use crate::codec::*;

pub struct MsgChannelOpenConfirmation<T: ChannelType> {
    pub recipient_channel: u32,
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub channel_type: T::Confirmation,
}

impl<'a, T: ChannelType> MsgChannelOpenConfirmation<T> {
    const MSG_NUMBER: u8 = 91;
}

impl <T: ChannelType> Encode for MsgChannelOpenConfirmation<T> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4 + 4
        + T::size_confirmation(&self.channel_type)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.sender_channel);
        e.push_u32be(self.initial_window_size);
        e.push_u32be(self.maximum_packet_size);
        T::encode_confirmation(&self.channel_type, e);
    }
}

impl<'a, T: ChannelType> Decode<'a> for MsgChannelOpenConfirmation<T> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            channel_type: T::decode_confirmation(d)?,
        }.into()
    }
}
