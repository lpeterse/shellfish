use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgServiceAccept<T = String>(T);

impl Message for MsgServiceAccept {
    const NUMBER: u8 = 6;
}

impl SshEncode for MsgServiceAccept<&'static str> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(MsgServiceAccept::NUMBER)?;
        e.push_str_framed(&self.0)
    }
}

impl SshDecode for MsgServiceAccept {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        SshDecode::decode(d).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgServiceAccept("service");
        assert_eq!(
            &[6, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 12] = [6, 0, 0, 0, 7, 115, 101, 114, 118, 105, 99, 101];
        let msg: MsgServiceAccept = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!("service", msg.0);
    }
}
