use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgRequestSuccess<'a> {
    pub data: &'a [u8],
}

impl<'a> Message for MsgRequestSuccess<'a> {
    const NUMBER: u8 = 81;
}

impl<'a> SshEncode for MsgRequestSuccess<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_bytes(&self.data)
    }
}

impl<'a> SshDecodeRef<'a> for MsgRequestSuccess<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            data: d.take_bytes_all()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgRequestSuccess { data: &b"data"[..] };
        assert_eq!(
            "MsgRequestSuccess { data: [100, 97, 116, 97] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgRequestSuccess { data: &b"data"[..] };
        assert_eq!(
            &[81, 100, 97, 116, 97][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [81, 100, 97, 116, 97];
        let msg: MsgRequestSuccess = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(&b"data"[..], msg.data);
    }
}
