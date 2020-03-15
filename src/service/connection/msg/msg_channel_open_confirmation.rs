use crate::codec::*;
use crate::message::*;

pub(crate) struct MsgChannelOpenConfirmation<'a> {
    pub recipient_channel: u32,
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub specific: &'a [u8],
}

impl<'a> Message for MsgChannelOpenConfirmation<'a> {
    const NUMBER: u8 = 91;
}

impl<'a> Encode for MsgChannelOpenConfirmation<'a> {
    fn size(&self) -> usize {
        1 + 4 + 4 + 4 + 4 + self.specific.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.sender_channel);
        e.push_u32be(self.initial_window_size);
        e.push_u32be(self.maximum_packet_size);
        e.push_bytes(&self.specific);
    }
}

impl<'a> DecodeRef<'a> for MsgChannelOpenConfirmation<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            specific: d.take_all()?,
        }
        .into()
    }
}
