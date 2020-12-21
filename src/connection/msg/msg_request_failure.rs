use crate::util::codec::*;
use crate::transport::Message;

#[derive(Debug)]
pub(crate) struct MsgRequestFailure;

impl Message for MsgRequestFailure {
    const NUMBER: u8 = 82;
}

impl Encode for MsgRequestFailure {
    fn size(&self) -> usize {
        1
    }
    fn encode<E: SshEncoder>(&self, e: &mut E)-> Option<()>  {
        e.push_u8(<Self as Message>::NUMBER)
    }
}

impl Decode for MsgRequestFailure {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgRequestFailure;
        assert_eq!("MsgRequestFailure", format!("{:?}", msg));
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgRequestFailure;
        assert_eq!(&[82][..], &SliceEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 1] = [82];
        let _: MsgRequestFailure = SliceDecoder::decode(&buf[..]).unwrap();
    }
}
