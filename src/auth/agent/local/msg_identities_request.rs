use crate::util::codec::*;
use crate::transport::Message;

#[derive(Debug, PartialEq)]
pub struct MsgIdentitiesRequest {}

impl Message for MsgIdentitiesRequest {
    const NUMBER: u8 = 11;
}

impl Encode for MsgIdentitiesRequest {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)
    }
}

impl Decode for MsgIdentitiesRequest {
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
        assert_eq!(1, Encode::size(& MsgIdentitiesRequest {}));
    }

    #[test]
    fn test_encode_01() {
        let mut buf = [0];
        let mut enc = SliceEncoder::new(buf.as_mut());
        assert_eq!(Encode::encode(& MsgIdentitiesRequest {}, &mut enc), Some(()));
        assert_eq!([11], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [11];
        let res = Some(MsgIdentitiesRequest {});
        assert_eq!(res, SliceDecoder::decode(buf.as_ref()));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        let res: Option<MsgIdentitiesRequest> = None;
        assert_eq!(res, SliceDecoder::decode(buf.as_ref()));
    }
}
