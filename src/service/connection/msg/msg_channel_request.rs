use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub(crate) struct MsgChannelRequest<'a, T> {
    pub recipient_channel: u32,
    pub request: &'a str,
    pub want_reply: bool,
    pub specific: T,
}

impl<'a, T> Message for MsgChannelRequest<'a, T> {
    const NUMBER: u8 = 98;
}

impl<'a, T: Encode> Encode for MsgChannelRequest<'a, T> {
    fn size(&self) -> usize {
        1 + 4 + Encode::size(&self.request) + 1 + Encode::size(&self.specific)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        e.push_u32be(self.recipient_channel);
        Encode::encode(&self.request, e);
        e.push_u8(self.want_reply as u8);
        Encode::encode(&self.specific, e);
    }
}

impl<'a> DecodeRef<'a> for MsgChannelRequest<'a, &'a [u8]> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            request: DecodeRef::decode(d)?,
            want_reply: d.take_u8()? != 0,
            specific: d.take_all()?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let x = MsgChannelRequest {
            recipient_channel: 23,
            request: "request",
            want_reply: true,
            specific: "specific",
        };
        assert_eq!(format!("{:?}", x), "MsgChannelRequest { recipient_channel: 23, request: \"request\", want_reply: true, specific: \"specific\" }");
    }

    #[test]
    fn test_encode_01() {
        let x = MsgChannelRequest {
            recipient_channel: 23,
            request: "request",
            want_reply: true,
            specific: "specific",
        };
        let actual = BEncoder::encode(&x);
        let expected = [
            98, 0, 0, 0, 23, 0, 0, 0, 7, 114, 101, 113, 117, 101, 115, 116, 1, 0, 0, 0, 8, 115,
            112, 101, 99, 105, 102, 105, 99,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_decode_01() {
        let x: MsgChannelRequest<&[u8]> = BDecoder::decode(
            &[
                98, 0, 0, 0, 23, 0, 0, 0, 7, 114, 101, 113, 117, 101, 115, 116, 1, 0, 0, 0, 8, 115,
                112, 101, 99, 105, 102, 105, 99,
            ][..],
        )
        .unwrap();
        assert_eq!(x.recipient_channel, 23);
        assert_eq!(x.request, "request");
        assert_eq!(x.want_reply, true);
    }
}
