use crate::codec::*;
use crate::message::*;

#[derive(Debug)]
pub(crate) struct MsgRequestSuccess<'a> {
    pub data: &'a [u8],
}

impl<'a> MsgRequestSuccess<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }
}

impl<'a> Message for MsgRequestSuccess<'a> {
    const NUMBER: u8 = 81;
}

impl<'a> Encode for MsgRequestSuccess<'a> {
    fn size(&self) -> usize {
        1 + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        e.push_bytes(&self.data);
    }
}

impl<'a> DecodeRef<'a> for MsgRequestSuccess<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            data: d.take_all()?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgRequestSuccess { data: &b"data"[..] };
        assert_eq!(
            "MsgRequestSuccess { data: [100, 97, 116, 97] }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgRequestSuccess { data: &b"data"[..] };
        assert_eq!(&[81, 100, 97, 116, 97][..], &BEncoder::encode(&msg)[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 5] = [81, 100, 97, 116, 97];
        let msg: MsgRequestSuccess = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(&b"data"[..], msg.data);
    }
}
