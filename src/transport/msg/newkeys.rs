use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgNewkeys;

impl Message for MsgNewkeys {
    const NUMBER: u8 = 21;
}

impl SshEncode for MsgNewkeys {
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(<Self as Message>::NUMBER)
    }
}

impl SshDecode for MsgNewkeys {
    fn decode<'a, D: SshDecoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgNewkeys {};
        assert_eq!(&[21][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [21];
        let _: MsgNewkeys = SshCodec::decode(&buf[..]).unwrap();
    }
}
