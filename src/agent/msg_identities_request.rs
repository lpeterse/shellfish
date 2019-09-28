use crate::codec::*;

#[derive(Debug, PartialEq)]
pub struct MsgIdentitiesRequest {}

impl MsgIdentitiesRequest {
    pub const MSG_NUMBER: u8 = 11;
}

impl Encode for MsgIdentitiesRequest {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
    }
}

impl <'a> DecodeRef<'a> for MsgIdentitiesRequest {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::MSG_NUMBER)?;
        Self {}.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size_01() {
        assert_eq!(1, Encode::size(& MsgIdentitiesRequest {}));
    }

    #[test]
    fn test_encode_01() {
        let mut buf = [0];
        let mut enc = BEncoder::from(buf.as_mut());
        Encode::encode(& MsgIdentitiesRequest {}, &mut enc);
        assert_eq!([11], buf);
    }

    #[test]
    fn test_decode_01() {
        let buf = [11];
        let mut dec = BDecoder::from(buf.as_ref());
        let res = Some(MsgIdentitiesRequest {});
        assert_eq!(res, DecodeRef::decode(&mut dec));
    }

    #[test]
    fn test_decode_02() {
        let buf = [0];
        let mut dec = BDecoder::from(buf.as_ref());
        let res: Option<MsgIdentitiesRequest> = None;
        assert_eq!(res, DecodeRef::decode(&mut dec));
    }
}
