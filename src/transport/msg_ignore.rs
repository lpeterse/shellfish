use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgIgnore<'a> {
    pub data: &'a [u8],
}

impl<'a> MsgIgnore<'a> {
    pub fn new() -> Self {
        Self { data: &[] }
    }
}

impl<'a> Message for MsgIgnore<'a> {
    const NUMBER: u8 = 2;
}

impl<'a> Encode for MsgIgnore<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(self.data)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(self.data, c);
    }
}

impl<'a> DecodeRef<'a> for MsgIgnore<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            data: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgIgnore { data: &b"data"[..] };
        assert_eq!(
            "MsgIgnore { data: [100, 97, 116, 97] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgIgnore::new();
        assert_eq!(
            &[2, 0, 0, 0, 0][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgIgnore { data: &b"data"[..] };
        assert_eq!(
            &[2, 0, 0, 0, 4, 100, 97, 116, 97][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 9] = [2, 0, 0, 0, 4, 100, 97, 116, 97];
        let msg: MsgIgnore = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(&b"data"[..], msg.data);
    }
}
