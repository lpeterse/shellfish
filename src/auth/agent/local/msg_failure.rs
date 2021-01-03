use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug, PartialEq)]
pub struct MsgFailure;

impl Message for MsgFailure {
    const NUMBER: u8 = 5;
}

impl SshEncode for MsgFailure {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)
    }
}

impl SshDecode for MsgFailure {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let mut buf = [0; 1];
        let mut enc = RefEncoder::new(buf.as_mut());
        assert_eq!(SshEncode::encode(&MsgFailure {}, &mut enc), Some(()));
        assert_eq!([5], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [5];
        let res = Some(MsgFailure {});
        assert_eq!(res, SshCodec::decode(buf.as_ref()));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        let res: Option<MsgFailure> = None;
        assert_eq!(res, SshCodec::decode(buf.as_ref()));
    }
}
