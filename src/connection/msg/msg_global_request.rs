use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgGlobalRequest<'a> {
    pub name: &'a str,
    pub data: &'a [u8],
    pub want_reply: bool,
}

impl <'a> Message for MsgGlobalRequest<'a> {
    const NUMBER: u8 = 80;
}

impl <'a> SshEncode for MsgGlobalRequest<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_str_framed(self.name)?;
        e.push_u8(self.want_reply as u8)?;
        e.push_bytes(self.data)
    }
}

impl <'a> SshDecodeRef<'a> for MsgGlobalRequest<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            name: d.take_str_framed()?,
            want_reply: d.take_bool()?,
            data: d.take_bytes_all()?,
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
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 14] = [80, 0, 0, 0, 4, 110, 97, 109, 101, 1, 100, 97, 116, 97];
        let msg: MsgGlobalRequest = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.name, "name");
        assert_eq!(msg.want_reply, true);
        assert_eq!(msg.data, &b"data"[..]);
    }
}
