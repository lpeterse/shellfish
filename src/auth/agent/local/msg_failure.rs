use crate::util::codec::*;
use crate::transport::Message;

#[derive(Debug, PartialEq)]
pub struct MsgFailure {}

impl Message for MsgFailure {
    const NUMBER: u8 = 5;
}

impl Encode for MsgFailure {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)
    }
}

impl Decode for MsgFailure {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {}.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_01() {
        assert_eq!(1, Encode::size(& MsgFailure {}));
    }

    #[test]
    fn test_encode_01() {
        let mut buf = [0;1];
        let mut enc = SliceEncoder::new(buf.as_mut());
        assert_eq!(Encode::encode(& MsgFailure {}, &mut enc), Some(()));
        assert_eq!([5], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [5];
        let res = Some(MsgFailure {});
        assert_eq!(res, SliceDecoder::decode(buf.as_ref()));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        let res: Option<MsgFailure> = None;
        assert_eq!(res, SliceDecoder::decode(buf.as_ref()));
    }
}
