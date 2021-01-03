use crate::transport::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgSuccess;

impl Message for MsgSuccess {
    const NUMBER: u8 = 52;
}

impl SshEncode for MsgSuccess {
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(<Self as Message>::NUMBER)
    }
}

impl SshDecode for MsgSuccess {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgSuccess {};
        assert_eq!("MsgSuccess", format!("{:?}", msg));
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgSuccess {};
        assert_eq!(&[52][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [52];
        let _: MsgSuccess = SshCodec::decode(&buf[..]).unwrap();
    }
}
