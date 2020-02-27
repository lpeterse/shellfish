use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgSuccess {}

impl Message for MsgSuccess {
    const NUMBER: u8 = 52;
}

impl Encode for MsgSuccess {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER as u8);
    }
}

impl Decode for MsgSuccess {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {}.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgSuccess {};
        assert_eq!(
            "MsgSuccess",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgSuccess {};
        assert_eq!(&[52][..], &BEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [52];
        let _: MsgSuccess = BDecoder::decode(&buf[..]).unwrap();
    }
}
