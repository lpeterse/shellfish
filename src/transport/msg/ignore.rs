use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgIgnore<'a> {
    pub data: &'a [u8],
}

impl<'a> Message for MsgIgnore<'a> {
    const NUMBER: u8 = 2;
}

impl<'a> SshEncode for MsgIgnore<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_bytes_framed(&self.data)
    }
}

impl<'a> SshDecodeRef<'a> for MsgIgnore<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        d.take_bytes_framed().map(|data| Self { data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgIgnore { data: &b"data"[..] };
        assert_eq!(
            "MsgIgnore { data: [100, 97, 116, 97] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgIgnore { data: b"" };
        assert_eq!(&[2, 0, 0, 0, 0][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgIgnore { data: &b"data"[..] };
        assert_eq!(
            &[2, 0, 0, 0, 4, 100, 97, 116, 97][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 9] = [2, 0, 0, 0, 4, 100, 97, 116, 97];
        let msg: MsgIgnore = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(&b"data"[..], msg.data);
    }
}
