use crate::transport::Message;
use crate::util::codec::*;

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
        1 + 4 + 4 + self.request.len() + 1 + self.specific.size()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_str_framed(&self.request)?;
        e.push_u8(self.want_reply as u8)?;
        e.push(&self.specific)
    }
}

impl<'a> DecodeRef<'a> for MsgChannelRequest<'a, &'a [u8]> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            request: DecodeRef::decode(d)?,
            want_reply: d.take_u8()? != 0,
            specific: d.take_bytes_all()?,
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
            specific: (),
        };
        let actual = SliceEncoder::encode(&x);
        let expected = [
            98, 0, 0, 0, 23, 0, 0, 0, 7, 114, 101, 113, 117, 101, 115, 116, 1,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_decode_01() {
        let x: MsgChannelRequest<&[u8]> = SliceDecoder::decode(
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
