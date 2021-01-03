use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgRequestFailure;

impl Message for MsgRequestFailure {
    const NUMBER: u8 = 82;
}

impl SshEncode for MsgRequestFailure {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)
    }
}

impl SshDecode for MsgRequestFailure {
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
        let msg = MsgRequestFailure;
        assert_eq!("MsgRequestFailure", format!("{:?}", msg));
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgRequestFailure;
        assert_eq!(&[82][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [82];
        let _: MsgRequestFailure = SshCodec::decode(&buf[..]).unwrap();
    }
}
