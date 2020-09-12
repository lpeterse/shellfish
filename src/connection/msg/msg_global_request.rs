use crate::util::codec::*;
use crate::transport::Message;

#[derive(Debug)]
pub(crate) struct MsgGlobalRequest {
    pub name: String,
    pub data: Vec<u8>,
    pub want_reply: bool,
}

impl MsgGlobalRequest {
    pub fn new<N: Into<String>, D: Into<Vec<u8>>>(name: N, data: D, want_reply: bool) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            want_reply,
        }
    }
}

impl Message for MsgGlobalRequest {
    const NUMBER: u8 = 80;
}

impl Encode for MsgGlobalRequest {
    fn size(&self) -> usize {
        1 + Encode::size(&self.name) + 1 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        Encode::encode(&self.name, e)?;
        e.push_u8(self.want_reply as u8)?;
        e.push_bytes(&self.data)
    }
}

impl Decode for MsgGlobalRequest {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            name: DecodeRef::decode(d)?,
            want_reply: d.take_u8()? != 0,
            data: d.take_all()?.into(),
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgGlobalRequest {
            name: "name".into(),
            want_reply: true,
            data: b"data"[..].into(),
        };
        assert_eq!(
            "MsgGlobalRequest { name: \"name\", data: [100, 97, 116, 97], want_reply: true }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgGlobalRequest {
            name: "name".into(),
            want_reply: true,
            data: b"data"[..].into(),
        };
        assert_eq!(
            &[80, 0, 0, 0, 4, 110, 97, 109, 101, 1, 100, 97, 116, 97][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 14] = [80, 0, 0, 0, 4, 110, 97, 109, 101, 1, 100, 97, 116, 97];
        let msg: MsgGlobalRequest = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.name, "name");
        assert_eq!(msg.want_reply, true);
        assert_eq!(msg.data, &b"data"[..]);
    }
}
