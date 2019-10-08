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

#[derive(Debug)]
pub struct MsgChannelRequest2<'a> {
    pub recipient_channel: u32,
    pub request: &'a str,
    pub want_reply: bool,
    pub specific: &'a [u8],
}

impl<'a> MsgChannelRequest2<'a> {
    const MSG_NUMBER: u8 = 98;
}

// FIXME
impl <'a> Encode for MsgChannelRequest2<'a> {
    fn size(&self) -> usize {
        0
    }
    fn encode<E: Encoder>(&self, _: &mut E) {
    }
}

impl<'a> DecodeRef<'a> for MsgChannelRequest2<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::MSG_NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            request: DecodeRef::decode(d)?,
            want_reply: d.take_u8()? != 0,
            specific: d.take_all()?,
        }.into()
    }
}
