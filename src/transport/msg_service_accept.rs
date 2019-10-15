use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgServiceAccept<'a>(&'a str);

impl<'a> Message for MsgServiceAccept<'a> {
    const NUMBER: u8 = 6;
}

impl<'a> Encode for MsgServiceAccept<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.0, c);
    }
}

impl<'a> DecodeRef<'a> for MsgServiceAccept<'a> {
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
        let msg = MsgServiceAccept(&"service");
        assert_eq!(
            "MsgServiceAccept(\"service\")",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgServiceAccept(&"service");
        assert_eq!(
            &[6, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 12] = [6, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101];
        let msg: MsgServiceAccept = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!("service", msg.0);
    }
}
