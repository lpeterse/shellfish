use crate::codec::*;
use crate::transport::Message;

#[derive(Debug, PartialEq)]
pub struct MsgSuccess {}

impl Message for MsgSuccess {
    const NUMBER: u8 = 6;
}

impl Encode for MsgSuccess {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
    }
}

impl Decode for MsgSuccess {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {}.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size_01() {
        assert_eq!(1, Encode::size(& MsgSuccess {}));
    }

    #[test]
    fn test_encode_01() {
        let mut buf = [0;1];
        let mut enc = BEncoder::from(buf.as_mut());
        Encode::encode(& MsgSuccess {}, &mut enc);
        assert_eq!([6], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [6];
        let mut dec = BDecoder::from(buf.as_ref());
        let res = Some(MsgSuccess {});
        assert_eq!(res, DecodeRef::decode(&mut dec));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        let mut dec = BDecoder::from(buf.as_ref());
        let res: Option<MsgSuccess> = None;
        assert_eq!(res, DecodeRef::decode(&mut dec));
    }
}
