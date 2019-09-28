use super::*;
use crate::codec::*;

#[derive(Debug)]
pub struct MsgChannelRequest<T: ChannelRequest> {
    pub recipient_channel: u32,
    pub want_reply: bool,
    pub request: T,
}

impl<T: ChannelRequest> MsgChannelRequest<T> {
    const MSG_NUMBER: u8 = 98;
}

impl<T: ChannelRequest + Encode> Encode for MsgChannelRequest<T> {
    fn size(&self) -> usize {
        1 + 4 + Encode::size(&self.request.name()) + 1 + Encode::size(&self.request)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER);
        e.push_u32be(self.recipient_channel);
        Encode::encode(&self.request.name(), e);
        e.push_u8(self.want_reply as u8);
        Encode::encode(&self.request, e);
    }
}

/*
impl<T> Decode<T> for MsgChannelRequest<T> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| *x == Self::MSG_NUMBER)?;
        Self {
            name: Decode::decode(d)?,
            want_reply: d.take_u8()? != 0,
            data: d.take_all()?,
        }.into()
    }
}
*/
