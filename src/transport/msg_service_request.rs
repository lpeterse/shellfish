use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgServiceRequest<'a>(pub &'a str);

impl<'a> Message for MsgServiceRequest<'a> {
    const NUMBER: u8 = 5;
}

impl<'a> Encode for MsgServiceRequest<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.0, c);
    }
}

impl<'a> DecodeRef<'a> for MsgServiceRequest<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self(DecodeRef::decode(d)?).into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgServiceRequest(&"service");
        assert_eq!(
            "MsgServiceRequest(\"service\")",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgServiceRequest(&"service");
        assert_eq!(
            &[5, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 12] = [5, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101];
        let msg: MsgServiceRequest = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!("service", msg.0);
    }
}
