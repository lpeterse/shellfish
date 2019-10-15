use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgDebug<'a> {
    pub always_display: bool,
    pub message: &'a str,
    pub language: &'a str,
}

impl<'a> Message for MsgDebug<'a> {
    const NUMBER: u8 = 4;
}

impl<'a> Encode for MsgDebug<'a> {
    fn size(&self) -> usize {
        1 + 1 + Encode::size(&self.message) + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER as u8);
        c.push_u8(self.always_display as u8);
        Encode::encode(&self.message, c);
        Encode::encode(&self.language, c);
    }
}

impl<'a> DecodeRef<'a> for MsgDebug<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            always_display: d.take_u8()? != 0,
            message: DecodeRef::decode(d)?,
            language: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod test {
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
            &BEncoder::encode(&msg)[..]
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
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 17] = [
            4, 23, 0, 0, 0, 3, 109, 115, 103, 0, 0, 0, 4, 108, 97, 110, 103,
        ];
        let msg: MsgDebug = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(true, msg.always_display);
        assert_eq!("msg", msg.message);
        assert_eq!("lang", msg.language);
    }

    #[test]
    fn test_decode_02() {
        let buf: [u8; 12] = [4, 0, 0, 0, 0, 1, 109, 0, 0, 0, 1, 108];
        let msg: MsgDebug = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(false, msg.always_display);
        assert_eq!("m", msg.message);
        assert_eq!("l", msg.language);
    }
}
