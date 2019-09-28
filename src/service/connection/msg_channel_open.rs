pub use super::*;

#[derive(Clone, Debug)]
pub struct MsgChannelOpen<T: ChannelType> {
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub channel_type: T::Open,
}

impl<'a,T: ChannelType> MsgChannelOpen<T> {
    const MSG_NUMBER: u8 = 90;
}

impl <T: ChannelType> Encode for MsgChannelOpen<T> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4
        + Encode::size(&<T as ChannelType>::NAME)
        + Encode::size(&self.channel_type)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&<T as ChannelType>::NAME, e);
        e.push_u32be(self.sender_channel);
        e.push_u32be(self.initial_window_size);
        e.push_u32be(self.maximum_packet_size);
        Encode::encode(&self.channel_type, e);
    }
}

impl<T: ChannelType> Decode for MsgChannelOpen<T> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        let _: &str = DecodeRef::decode(d).filter(|x| x == &<T as ChannelType>::NAME)?;
        Self {
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            channel_type: Decode::decode(d)?,
        }.into()
    }
}
