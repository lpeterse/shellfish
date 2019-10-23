use super::*;
use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgNewKeys {}

impl Message for MsgNewKeys {
    const NUMBER: u8 = 21;
}

impl Encode for MsgNewKeys {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
    }
}

impl Decode for MsgNewKeys {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {})
    }
}


#[cfg(test)]
mod test {
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
        assert_eq!(&[21][..], &BEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [21];
        let _: MsgNewKeys = BDecoder::decode(&buf[..]).unwrap();
    }
}
