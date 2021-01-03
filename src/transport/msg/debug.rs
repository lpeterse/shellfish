use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgDebug<'a> {
    pub always_display: bool,
    pub message: &'a str,
    pub language: &'a str,
}

impl<'a> Message for MsgDebug<'a> {
    const NUMBER: u8 = 4;
}

impl<'a> SshEncode for MsgDebug<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_bool(self.always_display)?;
        e.push_str_framed(&self.message)?;
        e.push_str_framed(&self.language)
    }
}

impl<'a> SshDecodeRef<'a> for MsgDebug<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            always_display: d.take_bool()?,
            message: d.take_str_framed()?,
            language: d.take_str_framed()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgDebug {
            always_display: true,
            message: "msg",
            language: "lang",
        };
        assert_eq!(
            "MsgDebug { always_display: true, message: \"msg\", language: \"lang\" }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgDebug {
            always_display: true,
            message: "msg",
            language: "lang",
        };
        assert_eq!(
            &[4, 1, 0, 0, 0, 3, 109, 115, 103, 0, 0, 0, 4, 108, 97, 110, 103][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgDebug {
            always_display: false,
            message: "m",
            language: "l",
        };
        assert_eq!(
            &[4, 0, 0, 0, 0, 1, 109, 0, 0, 0, 1, 108][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 17] = [
            4, 23, 0, 0, 0, 3, 109, 115, 103, 0, 0, 0, 4, 108, 97, 110, 103,
        ];
        let msg: MsgDebug = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(true, msg.always_display);
        assert_eq!("msg", msg.message);
        assert_eq!("lang", msg.language);
    }

    #[test]
    fn test_decode_02() {
        let buf: [u8; 12] = [4, 0, 0, 0, 0, 1, 109, 0, 0, 0, 1, 108];
        let msg: MsgDebug = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(false, msg.always_display);
        assert_eq!("m", msg.message);
        assert_eq!("l", msg.language);
    }
}
