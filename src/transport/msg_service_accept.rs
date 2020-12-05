use crate::util::codec::*;
use crate::transport::Message;

#[derive(Clone, Debug)]
pub struct MsgServiceAccept<T = String>(T);

impl Message for MsgServiceAccept {
    const NUMBER: u8 = 6;
}

impl Encode for MsgServiceAccept<&'static str> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(MsgServiceAccept::NUMBER)?;
        Encode::encode(&self.0, c)
    }
}

impl Decode for MsgServiceAccept {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self(Decode::decode(d)?).into()
    }
}

#[cfg(test)]
mod tests {
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
        let msg = MsgServiceAccept("service");
        assert_eq!(
            &[6, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 12] = [6, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101];
        let msg: MsgServiceAccept = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!("service", msg.0);
    }
}
