use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgNewKeys;

impl Message for MsgNewKeys {
    const NUMBER: u8 = 21;
}

impl SshEncode for MsgNewKeys {
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(<Self as Message>::NUMBER)
    }
}

impl SshDecode for MsgNewKeys {
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
        let msg = MsgNewKeys {};
        assert_eq!(&[21][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [21];
        let _: MsgNewKeys = SshCodec::decode(&buf[..]).unwrap();
    }
}
