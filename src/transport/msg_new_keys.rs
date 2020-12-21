use super::*;
use crate::util::codec::*;
use crate::transport::Message;

#[derive(Clone, Debug)]
pub struct MsgNewKeys;

impl Message for MsgNewKeys {
    const NUMBER: u8 = 21;
}

impl Encode for MsgNewKeys {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(<Self as Message>::NUMBER)
    }
}

impl Decode for MsgNewKeys {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgNewKeys {};
        assert_eq!(
            "MsgNewKeys",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgNewKeys {};
        assert_eq!(&[21][..], &SliceEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [21];
        let _: MsgNewKeys = SliceDecoder::decode(&buf[..]).unwrap();
    }
}
