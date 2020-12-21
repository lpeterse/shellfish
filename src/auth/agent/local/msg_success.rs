use crate::util::codec::*;
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
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)
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
    fn test_size_01() {
        assert_eq!(1, Encode::size(& MsgSuccess {}));
    }

    #[test]
    fn test_encode_01() {
        let mut buf = [0;1];
        let mut enc = SliceEncoder::new(buf.as_mut());
        assert_eq!(Encode::encode(& MsgSuccess {}, &mut enc), Some(()));
        assert_eq!([6], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [6];
        let res = Some(MsgSuccess {});
        assert_eq!(res, SliceDecoder::decode(buf.as_ref()));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        let res: Option<MsgSuccess> = None;
        assert_eq!(res, SliceDecoder::decode(buf.as_ref()));
    }
}
