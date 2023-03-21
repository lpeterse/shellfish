use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgChannelOpenConfirmation<'a> {
    pub recipient_channel: u32,
    pub sender_channel: u32,
    pub initial_window_size: u32,
    pub maximum_packet_size: u32,
    pub specific: &'a [u8],
}

impl<'a> MsgChannelOpenConfirmation<'a> {
    pub fn new(
        recipient_channel: u32,
        sender_channel: u32,
        initial_window_size: u32,
        maximum_packet_size: u32,
    ) -> Self {
        Self {
            recipient_channel,
            sender_channel,
            initial_window_size,
            maximum_packet_size,
            specific: &[],
        }
    }
}

impl<'a> Message for MsgChannelOpenConfirmation<'a> {
    const NUMBER: u8 = 91;
}

impl<'a> SshEncode for MsgChannelOpenConfirmation<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_u32be(self.sender_channel)?;
        e.push_u32be(self.initial_window_size)?;
        e.push_u32be(self.maximum_packet_size)?;
        e.push_bytes(&self.specific)
    }
}

impl<'a> SshDecodeRef<'a> for MsgChannelOpenConfirmation<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            recipient_channel: d.take_u32be()?,
            sender_channel: d.take_u32be()?,
            initial_window_size: d.take_u32be()?,
            maximum_packet_size: d.take_u32be()?,
            specific: d.take_bytes_all()?,
        })
    }
}
