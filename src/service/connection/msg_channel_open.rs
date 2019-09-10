pub use super::*;

#[derive(Clone, Debug)]
pub struct MsgChannelOpen<T: ChannelType> {
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub channel_type: T::Request,
}

impl<'a,T: ChannelType> MsgChannelOpen<T> {
    const MSG_NUMBER: u8 = 90;
}

impl<'a,T: ChannelType> Codec<'a> for MsgChannelOpen<T> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4
        + Codec::size(&T::name())
        + T::size_request(&self.channel_type)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&T::name(), e);
        e.push_u32be(self.sender_channel);
        e.push_u32be(self.initial_window_size);
        e.push_u32be(self.maximum_packet_size);
        T::encode_request(&self.channel_type, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        let _: &str = Codec::decode(d).filter(|x| x == &T::name())?;
        Self {
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            channel_type: T::decode_request(d)?,
        }.into()
    }
}
