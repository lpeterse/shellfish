use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub struct MsgGlobalRequest<'a> {
    pub name: &'a str,
    pub want_reply: bool,
    pub data: &'a [u8],
}

impl<'a> Message for MsgGlobalRequest<'a> {
    const NUMBER: u8 = 80;
}

impl<'a> Encode for MsgGlobalRequest<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.name) + 1 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.name, e);
        e.push_u8(self.want_reply as u8);
        e.push_bytes(&self.data);
    }
}

impl<'a> DecodeRef<'a> for MsgGlobalRequest<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            name: DecodeRef::decode(d)?,
            want_reply: d.take_u8()? != 0,
            data: d.take_all()?,
        }
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgGlobalRequest {
            name: "name",
            want_reply: true,
            data: &b"data"[..],
        };
        assert_eq!(
            "MsgGlobalRequest { name: \"name\", want_reply: true, data: [100, 97, 116, 97] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgGlobalRequest {
            name: "name",
            want_reply: true,
            data: &b"data"[..],
        };
        assert_eq!(
            &[80, 0, 0, 0, 4, 110, 97, 109, 101, 1, 100, 97, 116, 97][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 14] = [80, 0, 0, 0, 4, 110, 97, 109, 101, 1, 100, 97, 116, 97];
        let msg: MsgGlobalRequest = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.name, "name");
        assert_eq!(msg.want_reply, true);
        assert_eq!(msg.data, &b"data"[..]);
    }
}
