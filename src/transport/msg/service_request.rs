use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgServiceRequest<T = String>(pub T);

impl<T> Message for MsgServiceRequest<T> {
    const NUMBER: u8 = 5;
}

impl SshEncode for MsgServiceRequest<&str> {
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u8(<Self as Message>::NUMBER)?;
        c.push_str_framed(&self.0)
    }
}

impl SshDecode for MsgServiceRequest<String> {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        d.take_str_framed().map(String::from).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgServiceRequest::<&'static str>(&"service");
        assert_eq!(
            &[5, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 12] = [5, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101];
        let msg: MsgServiceRequest = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!("service", msg.0);
    }
}
