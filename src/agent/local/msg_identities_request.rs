use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug, PartialEq)]
pub struct MsgIdentitiesRequest;

impl Message for MsgIdentitiesRequest {
    const NUMBER: u8 = 11;
}

impl SshEncode for MsgIdentitiesRequest {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)
    }
}

impl SshDecode for MsgIdentitiesRequest {
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
        let mut buf = [0];
        let mut enc = RefEncoder::new(buf.as_mut());
        assert_eq!(
            SshEncode::encode(&MsgIdentitiesRequest {}, &mut enc),
            Some(())
        );
        assert_eq!([11], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [11];
        let res = Ok(MsgIdentitiesRequest {});
        assert_eq!(res, SshCodec::decode(buf.as_ref()));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        assert_eq!(
            SshCodec::decode::<MsgIdentitiesRequest>(buf.as_ref()).is_ok(),
            false
        );
    }
}
